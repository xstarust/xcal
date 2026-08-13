//! 太阳时间计算
//!
//! # 概念区分
//!
//! | 术语 | 含义 | 差异 |
//! |------|------|------|
//! | **标准时（Standard Time）** | 时区统一时间，如 UTC+8 | 基准 |
//! | **地方平均时（Local Mean Time）** | 经度校正后的时间 | 经度 × 4 分钟/度 |
//! | **真太阳时（Apparent Solar Time）** | LMT + 均时差 | ±16 分钟 |
//!
//! 均时差（Equation of Time）由地球椭圆轨道和黄赤交角共同引起，最大 ±16 分钟。

use crate::core::constants::{DAYS_PER_CENTURY, J2000, PI};
use crate::core::eph0::sun_data::{E_L0, E_L1, E_L2, E_L3, E_L4, E_L5};

/// 均时差（秒）
///
/// EOT = 太阳平黄经 − 太阳赤经（从真黄经经黄赤交角转换），转秒。
/// 平黄经用 IAU 多项式，真黄经用 VSOP87 6 层（几何位置，不加章动/光行差）。
/// 精度 ~0.5 秒。
pub fn equation_of_time_seconds(jd: f64) -> f64 {
    let t = (jd - J2000) / DAYS_PER_CENTURY;
    let tk = t / 10.0;

    // 平黄经（弧度）：IAU 多项式
    let l0 = (280.4664567 + 36000.76982779 * t + 0.0003032028 * t * t) * PI / 180.0;

    // 真黄经（弧度）：VSOP87 6 层，只加坐标基准修正（不加章动/光行差）
    fn vsop87_geo_lon(tk: f64) -> f64 {
        let mut v = 0.0;
        let mut tn = 1.0;
        for layer in [
            &E_L0[..],
            &E_L1[..],
            &E_L2[..],
            &E_L3[..],
            &E_L4[..],
            &E_L5[..],
        ] {
            let mut sum = 0.0;
            for item in layer.iter() {
                sum += item[0] * (item[1] + tk * item[2]).cos();
            }
            v += sum * tn;
            tn *= tk;
        }
        let t2 = tk * tk;
        let t3 = t2 * tk;
        v += (-0.0728 - 2.7702 * tk - 1.1019 * t2 - 0.0996 * t3) / 206264.80624709636;
        (v + PI).rem_euclid(2.0 * PI)
    }
    let true_lon = vsop87_geo_lon(tk);

    // 黄赤交角（弧度）
    let eps = (23.4392911 - 0.013004167 * t) * PI / 180.0;

    // 真黄经 → 赤经
    let ra = (true_lon.sin() * eps.cos()).atan2(true_lon.cos());

    // 平黄经 − 赤经，归一化到 [-π, π]
    let mut diff = (l0 - ra).rem_euclid(2.0 * PI);
    if diff > PI {
        diff -= 2.0 * PI;
    }

    // 弧度→秒
    diff * 86400.0 / (2.0 * PI)
}

/// 地方平均时（Local Mean Time）
///
/// 只做经度校正：标准时间 → 当地经度的平均时间。
/// LMT = 标准时 + (经度 − 标准子午线) × 4 分钟/度
/// **不含均时差**，如需真太阳时请使用 [`apparent_solar_time`]。
pub fn local_mean_time(jd: f64, longitude: f64) -> (i32, u32, u32) {
    let standard_meridian = (longitude / 15.0).round() * 15.0;
    let correction_seconds = (longitude - standard_meridian) / 15.0 * 3600.0;
    let jd_adjusted = jd + correction_seconds / 86400.0;

    // JD starts at noon; shift to civil midnight before extracting the clock.
    let fraction = (jd_adjusted + 0.5).rem_euclid(1.0);
    let total_seconds = (fraction * 86400.0).round() as i64;
    let norm = total_seconds.rem_euclid(86400);

    let hour = norm / 3600;
    let minute = (norm % 3600) / 60;
    let second = norm % 60;

    (hour as i32, minute as u32, second as u32)
}

/// 真太阳时（Apparent Solar Time）
///
/// LMT + 均时差校正。这是日晷指示的"真实"太阳时刻。
/// 与 `local_mean_time` 相差 ±16 分钟。
pub fn apparent_solar_time(jd: f64, longitude: f64) -> (i32, u32, u32) {
    let (h, m, s) = local_mean_time(jd, longitude);
    let lmt_seconds = h as f64 * 3600.0 + m as f64 * 60.0 + s as f64;
    let eot_seconds = equation_of_time_seconds(jd);

    let total_seconds = (lmt_seconds + eot_seconds).round() as i64;
    let norm = total_seconds.rem_euclid(86400);

    let hour = norm / 3600;
    let minute = (norm % 3600) / 60;
    let second = norm % 60;

    (hour as i32, minute as u32, second as u32)
}

/// 标准时 → 真太阳时（Apparent Solar Time）
///
/// 输入北京时间等时区标准时间，输出当地日晷指示的"真实"太阳时刻。
/// 校正链路：标准时 → LMT（经度校正，±30 分）→ EOT（均时差，±16 分）
/// 自动处理跨日回绕。
///
/// # Returns
/// `(year, month, day, hour, minute)` — 真太阳时日期时间
pub fn convert_to_apparent_solar_time(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    longitude: f64,
) -> (i32, u32, u32, u32, u32) {
    // 标准时 JD → LMT（经度校正）→ 加 EOT（均时差）
    let jd = crate::time::jd::JulianDay::from_ymd(year, month, day).0;
    let standard_meridian = (longitude / 15.0).round() * 15.0;
    let lmt_offset_seconds = (longitude - standard_meridian) / 15.0 * 3600.0;
    let jd_std = jd + hour as f64 / 24.0 + minute as f64 / 1440.0;
    let jd_lmt = jd_std + lmt_offset_seconds / 86400.0;
    let eot_seconds = equation_of_time_seconds(jd_lmt);
    let jd_apparent = jd_lmt + eot_seconds / 86400.0;

    let (ay, am, ad) = crate::time::jd::JulianDay(jd_apparent).to_ymd();
    let day_frac = jd_apparent - crate::time::jd::JulianDay::from_ymd(ay, am, ad).0;
    let total_minutes = (day_frac * 1440.0).round() as u32;
    (ay, am, ad, total_minutes / 60, total_minutes % 60)
}

/// 根据经度推算时区偏移（小时）
pub fn calc_utc_offset(longitude: f64) -> f64 {
    (longitude / 15.0).round()
}

/// 计算经度到标准子午线的校正量（秒）
pub fn calc_time_correction(longitude: f64, tz_offset: f64) -> i64 {
    let standard_meridian = tz_offset * 15.0;
    ((longitude - standard_meridian) / 15.0 * 3600.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eot_magnitude() {
        let jds = [2451545.0, 2460310.5, 2460482.5, 2460576.5];
        for &jd in &jds {
            let eot = equation_of_time_seconds(jd);
            assert!(eot.abs() < 1200.0, "EOT={} 超出范围", eot);
        }
    }

    #[test]
    fn test_eot_approximately_minus_3_minutes() {
        // 2000-01-01 附近 EOT ≈ -189 秒（~3 分钟）
        let eot = equation_of_time_seconds(2451545.0);
        assert!((eot + 189.0).abs() < 60.0, "eot={eot}");
    }

    #[test]
    fn test_eot_range_plausible() {
        // EOT 应在 ±1200 秒（±20 分）内，且符号随季节规律变化
        let data: &[(f64, &str)] = &[
            (2451545.0, "2000-01"),
            (2460493.5, "2024-07"),
            (2460978.5, "2025-11"),
        ];
        for &(jd, label) in data {
            let eot = equation_of_time_seconds(jd);
            assert!(eot.abs() < 1200.0, "{label}: EOT={eot:.1}s 超出范围");
        }
    }

    #[test]
    fn test_lmt_uses_midnight_based_clock() {
        // JD .5 is civil midnight; JD .0 is civil noon.
        assert_eq!(local_mean_time(2451544.5, 120.0), (0, 0, 0));
        assert_eq!(local_mean_time(2451545.0, 120.0), (12, 0, 0));
    }

    #[test]
    fn test_lmt_shanghai_six_minutes_ahead() {
        // 上海 121.5°E: JD=2460310.5 = 2024-01-01 00:00 UTC
        // LMT = 00:00 + (121.5-120)/15*3600 秒 = 00:06
        let jd = 2460310.5;
        let (h, m, _) = local_mean_time(jd, 121.5);
        assert_eq!((h, m), (0, 6));
    }

    #[test]
    fn test_lmt_normalizes_across_midnight() {
        assert_eq!(local_mean_time(2451544.5, 127.5), (23, 30, 0));
        assert_eq!(local_mean_time(2451544.5, 100.0), (23, 40, 0));
        assert_eq!(local_mean_time(2451544.5 - 1.0 / 48.0, 120.0), (23, 30, 0));
    }

    #[test]
    fn test_ast_uses_civil_clock() {
        let (hour, minute, _) = apparent_solar_time(2451544.5, 120.0);
        assert_eq!((hour, minute), (23, 56));
    }

    #[test]
    fn test_ast_differs_from_lmt_by_eot() {
        let jd = 2460310.5;
        let (lm_h, lm_m, _) = local_mean_time(jd, 120.0);
        let (as_h, as_m, _) = apparent_solar_time(jd, 120.0);
        let lmt = lm_h * 3600 + lm_m as i32 * 60;
        let ast = as_h * 3600 + as_m as i32 * 60;
        let diff = (ast - lmt + 43_200).rem_euclid(86_400) - 43_200;
        assert!(
            diff.abs() <= 1200,
            "LMT=({lm_h},{lm_m}) AST=({as_h},{as_m}) diff={diff}s"
        );
    }

    #[test]
    fn test_calc_utc_offset_east() {
        assert_eq!(calc_utc_offset(120.0), 8.0);
    }

    #[test]
    fn test_calc_utc_offset_west() {
        assert_eq!(calc_utc_offset(-75.0), -5.0);
    }

    #[test]
    fn test_calc_utc_offset_prime_meridian() {
        assert_eq!(calc_utc_offset(0.0), 0.0);
    }
}
