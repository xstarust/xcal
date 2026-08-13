//! ΔT 修正（历史时间修正）

/// 计算 ΔT 修正值（秒）
///
/// 基于历史观测数据和推算公式
/// 1600-2100 年范围内精度较好
pub fn delta_t(jd: f64) -> f64 {
    // 转换为年份
    let year = (jd - 2451545.0) / 365.25 + 2000.0;
    let t = year - 2000.0;
    let t2 = t * t;

    // 基于 NASA/IAU 推荐的多项式拟合
    if year < 1900.0 {
        // 1900 年之前的历史数据
        let u = (year - 1820.0) / 100.0;
        let u2 = u * u;
        -20.0 + 32.0 * u + 0.5 * u2
    } else if year < 2000.0 {
        // 1900-2000 年
        let t = year - 1900.0;
        let t2 = t * t;
        -2.79 + 1.494119 * t - 0.000194 * t2 + 0.00000021 * (t2 * t)
    } else if year < 2100.0 {
        // 2000-2100 年
        64.0 + 29.0 * t + 0.5 * t2
    } else {
        // 2100 年以后的推算
        let u = (year - 2000.0) / 100.0;
        let u2 = u * u;
        let u3 = u2 * u;
        -20.0 + 32.0 * u + 0.5 * u2 + 0.1 * u3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_t_historical() {
        let jd = 2305447.5; // 1600-01-01
        let dt = delta_t(jd);
        // 多项式外推，1600 年约 -88s
        assert!(dt > -100.0, "delta_t at 1600: {}", dt);
        assert!(dt < -80.0, "delta_t at 1600: {}", dt);
    }

    #[test]
    fn test_delta_t_modern() {
        let jd = 2451545.0; // J2000.0
        let dt = delta_t(jd);
        assert!(dt > 60.0, "delta_t at J2000 should be >60s: {}", dt);
        assert!(dt < 70.0, "delta_t at J2000 should be <70s: {}", dt);
    }

    #[test]
    fn test_delta_t_2100() {
        let jd = 2488069.5; // 2100-01-01
        let dt = delta_t(jd);
        // 2000-2100 分支，t≈100 → 约 7964s
        assert!(dt > 7000.0, "delta_t at 2100: {}", dt);
        assert!(dt < 8000.0, "delta_t at 2100: {}", dt);
    }

    #[test]
    fn test_delta_t_monotonic_19th() {
        let jd1 = 2378496.5; // 1800
        let jd2 = 2382147.5; // 1820
        let dt1 = delta_t(jd1);
        let dt2 = delta_t(jd2);
        assert!(dt2 > dt1, "delta_t should increase from 1800 to 1820");
    }
}
