//! 农历计算核心
//!
//! ### 日界对齐约定
//! 农历日界以中国标准时间（UTC+8）午夜为准。
//! 新月 JD 加 8 小时后再通过 `to_ymd()`（即 `floor(jd + 0.5)`）转为日历日期。
//!
//! ### 精度说明
//! 基于 ELP/MPP02 月亮模型（627 项）+ VSOP87 太阳模型（376 项），
//! 定朔定气法。已通过 UTC+8 日界对齐消除了边界日偏差。
//!
//! ### 算法

use super::historical;
use crate::core::eph0::{moon_sun_delta_deg, sun_longitude_deg};
use crate::time::jd::JulianDay;

/// 农历日期
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LunarDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub is_leap: bool,
}

impl LunarDate {
    pub fn new(year: i32, month: u32, day: u32, is_leap: bool) -> Self {
        Self {
            year,
            month,
            day,
            is_leap,
        }
    }
}

/// 纪元朔日: 2000-01-06 18:14 UTC ≈ JD 2451550.257
const EPOCH_SHUO_JD: f64 = 2451550.257;
/// 朔望月平均长度（天）
const SYNODIC_MONTH: f64 = 29.530588853;

/// 通过纪元索引独立找朔日，每次调用独立计算不累积误差
fn find_shuo(jd: f64) -> f64 {
    let n = ((jd - EPOCH_SHUO_JD) / SYNODIC_MONTH).round() as i32;
    let mut t = EPOCH_SHUO_JD + n as f64 * SYNODIC_MONTH;
    for _ in 0..10 {
        let d = moon_sun_delta_deg(t);
        if d.abs() < 1e-8 {
            break;
        }
        let ep = 0.01;
        let der = (moon_sun_delta_deg(t + ep) - moon_sun_delta_deg(t - ep)) / (2.0 * ep);
        if der.abs() < 1e-10 {
            break;
        }
        t -= d / der;
    }
    t
}

/// 计算气（节气/中气）发生的 JD
///
/// 气 = 太阳真黄经到达指定角度 `lon`（度）的时刻。
/// 配合 `find_shuo` 使用：朔定初一，气定月建和闰月。
fn find_qi(jd_approx: f64, lon: f64) -> f64 {
    let mut jd = jd_approx;
    for _ in 0..12 {
        let l = sun_longitude_deg(jd);
        let mut d = l - lon;
        d = d.rem_euclid(360.0);
        if d > 180.0 {
            d -= 360.0;
        }
        if d.abs() < 1e-6 {
            break;
        }
        jd -= d * 365.25 / 360.0;
    }
    jd
}

/// 内部月索引（冬至月=0）→ 农历月号（1=正月…12=腊月）
///
/// 农历以冬至所在月为一年的起点（十一月），内部编号为 0（月索引）。
///   0=十一月（冬至月）, 1=十二月, 2=正月, …, 11=十月, 12=十一月（下一周期）
///   映射公式：农历月号 = (idx + 10) % 12 + 1
///      0→11(冬), 1→12(腊), 2→1(正), …, 11→9, 12→10
///
/// 有闰月时，闰月占了一个额外位置，导致其后的月索引比实际月号大 1。
fn index_to_month(idx: usize, leap: Option<usize>, is_leap: bool) -> u32 {
    if is_leap {
        // 闰月与前一非闰月同号。
        // idx=lo 比实际月号大 1，所以 month(lo) = ((lo-1+10)%12+1) = ((lo+9)%12+1)
        ((idx + 9) % 12 + 1) as u32
    } else if let Some(lo) = leap {
        if idx > lo {
            // 闰月后的位置补偿：idx 多 1
            ((idx + 9) % 12 + 1) as u32
        } else {
            ((idx + 10) % 12 + 1) as u32
        }
    } else {
        ((idx + 10) % 12 + 1) as u32
    }
}

/// 农历月号（1=正月…12=腊月）→ 内部月索引（冬至月=0）
///
/// index_to_month 的逆运算。有闰月时，闰月后的月索引多出 1。
///
/// 朔日数组覆盖 15 个月跨越两个农历年，月索引 0 和 12 都代表十一月：
///   0/1 + lunar_year == anchor_year → 首年的十一月/十二月
///   12/13 + lunar_year > anchor_year → 次年的十一月/十二月
fn month_to_index(
    month: u32,
    leap: Option<usize>,
    is_leap: bool,
    lunar_year: i32,
    anchor_year: i32,
) -> usize {
    let base = ((month + 1) % 12) as usize;
    let mut idx = if is_leap {
        leap.unwrap_or(base)
    } else if let Some(lo) = leap {
        if base >= lo { base + 1 } else { base }
    } else {
        base
    };
    // idx=0/1(十一月/十二月)同时出现在两个位置：
    // 首轮 (idx=0/1) 属于 anchor_year，次轮 (idx=12/13) 属于 anchor_year+1
    if idx < 2 && lunar_year > anchor_year {
        idx += 12;
    }
    idx
}

/// 公历转农历
///
/// 算法核心思路：
/// 农历以冬至所在月为年首（十一月），因此需要先定位目标日期前后
/// 的冬至日。冬至所在的公历年份称为"锚定年"（anchor_year）。
/// 从锚定年的冬至开始，往前推 15 个连续朔日，生成 15 个月的朔日表。
/// 目标日期落在哪个朔日区间，就对应哪个农历月。
///
/// 使用 `floor(jd + 0.5)` 确定日历日。新月 JD 加 8 小时（中国标准时间
/// UTC+8）后再转日历日，使分界线为北京时间午夜而非 UT 午夜。
#[allow(clippy::collapsible_if)]
pub fn solar_to_lunar(year: i32, month: u32, day: u32) -> LunarDate {
    // 历史年份走修正表路径
    if year < 1900 {
        if let Some(l) = solar_to_lunar_historical(year, month, day) {
            return l;
        }
    }

    // UTC+8 偏移：8 小时 = 1/3 天
    const UTC8: f64 = 1.0 / 3.0;

    for anchor_year in (year - 1).max(1900)..=year.max(1900) {
        let dongzhi_jd = find_qi(JulianDay::from_ymd(anchor_year, 12, 1).0, 270.0);

        let shuo_idx = ((dongzhi_jd - EPOCH_SHUO_JD) / SYNODIC_MONTH - 1.0).ceil() as i32;

        // 生成 15 个连续朔日并转为 CST 日历日期
        let moons: Vec<(i32, u32, u32)> = (0..15)
            .map(|i| {
                JulianDay(find_shuo(EPOCH_SHUO_JD + (shuo_idx + i) as f64 * SYNODIC_MONTH) + UTC8)
                    .to_ymd()
            })
            .collect();

        // 检查目标日是否在范围内
        let (m0_y, m0_m, m0_d) = moons[0];
        let jd_m0 = JulianDay::from_ymd(m0_y, m0_m, m0_d).0;
        let jd_target = JulianDay::from_ymd(year, month, day).0;
        let next_dongzhi_jd = find_qi(dongzhi_jd + 360.0, 270.0);
        if jd_target < jd_m0 - 30.0 || jd_target > next_dongzhi_jd + 30.0 {
            continue;
        }

        // 闰月
        let leap = if moons.len() > 13 {
            (1..moons.len().saturating_sub(1))
                .find(|&i| find_mid_qi(&moons[i], &moons[i + 1]).is_none())
        } else {
            None
        };

        if let Some((_idx, is_leap, mn, yr, day)) =
            locate_in_lunar_context(year, month, day, &moons, leap, anchor_year)
        {
            return LunarDate::new(yr, mn, day, is_leap);
        }
    }

    LunarDate::new(year, month, day, false)
}

/// 检查一个朔望月内是否包含中气（用于闰月判定）
///
/// 遍历 12 个中气（黄经 30° 倍角），如有则返回中气索引，无则说明该月为闰月。
fn find_mid_qi(
    start: &(i32, u32, u32), // 本月朔日（初一）的公历日期
    end: &(i32, u32, u32),   // 下月朔日（初一）的公历日期
) -> Option<usize> {
    let start_jd = JulianDay::from_ymd(start.0, start.1, start.2).0;
    let end_jd = JulianDay::from_ymd(end.0, end.1, end.2).0;
    (0..12).find(|&q| {
        let tj = find_qi(start_jd, q as f64 * 30.0);
        tj >= start_jd && tj < end_jd
    })
}

/// 用朔日数组（moons）和闰月信息定位公历日期对应的农历日
///
/// moons 是一串连续的朔日日期，顺序排列：
///   moons[0] = 第 0 个朔日（十一月）
///   moons[1] = 第 1 个朔日（十二月）
///   ...
/// 目标日期 target 落在 moons[i-1] 和 moons[i] 之间，
/// 说明它属于第 i-1 个农历月。
///
/// 返回 Some(月索引, 是否闰月, 农历月号, 农历年, 当月的第几天)
fn locate_in_lunar_context(
    year: i32,
    month: u32,
    day: u32,
    moons: &[(i32, u32, u32)],
    leap: Option<usize>,
    anchor_year: i32,
) -> Option<(usize, bool, u32, i32, u32)> {
    // 月索引 → 农历年的偏移：idx=0/1（十一月/十二月）属 anchor_year，
    // idx>=2（正月~十月）属 anchor_year + 1
    let lunar_year = |idx| {
        if idx < 2 {
            anchor_year
        } else {
            anchor_year + 1
        }
    };
    // 月内天数 = 朔日到目标日的天数差 + 1
    let day_offset = |new_moon: &(i32, u32, u32)| {
        days_between(new_moon.0, new_moon.1, new_moon.2, year, month, day) + 1
    };

    let target = (year, month, day);
    for i in 1..moons.len() {
        if moons[i] > target {
            let idx = i - 1;
            let is_leap = leap == Some(idx);
            let day = day_offset(&moons[idx]);
            let month = index_to_month(idx, leap, is_leap);
            return Some((idx, is_leap, month, lunar_year(idx), day));
        }
    }
    // 目标 > 所有新月，尝试最后一个区间
    if moons.len() >= 2 {
        let idx = moons.len() - 2;
        let day = day_offset(&moons[idx]);
        let month = index_to_month(idx, leap, false);
        return Some((idx, false, month, lunar_year(idx), day));
    }
    None
}

/// 用历史算法（修正表）尝试转换公历到农历
fn solar_to_lunar_historical(year: i32, month: u32, day: u32) -> Option<LunarDate> {
    if !(-722..1900).contains(&year) {
        return None;
    }
    for anchor_year in (year - 1)..=year {
        if let Some((moons, leap, _era)) = historical::shuo_list(anchor_year) {
            let jd_target = JulianDay::from_ymd(year, month, day).0;
            let (m0_y, m0_m, m0_d) = moons[0];
            let jd_m0 = JulianDay::from_ymd(m0_y, m0_m, m0_d).0;
            let (ml_y, ml_m, ml_d) = moons[moons.len() - 1];
            let jd_last = JulianDay::from_ymd(ml_y, ml_m, ml_d).0;
            if jd_target < jd_m0 - 30.0 || jd_target > jd_last + 30.0 {
                continue;
            }
            if let Some((_idx, is_leap, mn, yr, day)) =
                locate_in_lunar_context(year, month, day, &moons, leap, anchor_year)
            {
                return Some(LunarDate::new(yr, mn, day, is_leap));
            }
        }
    }
    None
}

/// 计算从日期 a 到日期 b 的天数差
fn days_between(
    from_year: i32,
    from_month: u32,
    from_day: u32,
    to_year: i32,
    to_month: u32,
    to_day: u32,
) -> u32 {
    let jd_from = JulianDay::from_ymd(from_year, from_month, from_day).0;
    let jd_to = JulianDay::from_ymd(to_year, to_month, to_day).0;
    (jd_to - jd_from).round() as u32
}

/// 农历转公历
///
/// 逆向 `solar_to_lunar`。通过农历年月 + 闰月标记定位新月 JD，
/// 再加 (day - 1) 天得到公历日期。历史日期自动走修正表路径。
///
/// # 算法
/// 1. 根据农历月号推算所在冬季年（anchor_year）
/// 2. 生成该冬季年区间内 15 个连续朔日
/// 3. 找新月日历日 + (day - 1) → 公历日
pub fn lunar_to_solar(lunar: &LunarDate) -> (i32, u32, u32) {
    if lunar.month == 0 || lunar.month > 12 || lunar.day == 0 || lunar.day > 30 {
        return (lunar.year, lunar.month, lunar.day);
    }

    // UTC+8 偏移：农历日界以北京时间午夜为准
    const UTC8: f64 = 1.0 / 3.0;

    // 确定 anchor_year：
    //   month=1..10（正月~十月）：anchor_year = lunar_year - 1
    //   month=11,12（十一月/十二月）：anchor_year = lunar_year
    // 因为 idx=0/1（十一月/十二月）的 yr = anchor_year，
    // 而 idx=2+（正月~十月）的 yr = anchor_year + 1
    let anchor_year = if lunar.month >= 1 && lunar.month <= 10 {
        // month=1(正月)→idx=2→yr=wy+1→lunar_year=wy+1→wy=lunar_year-1
        lunar.year - 1
    } else {
        // month=11(十一月)→idx=0→yr=wy→lunar_year=wy→wy=lunar_year
        // month=12(十二月)→idx=1→yr=wy→lunar_year=wy→wy=lunar_year
        lunar.year
    };

    let dongzhi_jd = find_qi(JulianDay::from_ymd(anchor_year, 12, 1).0, 270.0);
    let shuo_idx = ((dongzhi_jd - EPOCH_SHUO_JD) / SYNODIC_MONTH - 1.0).ceil() as i32;

    let moons: Vec<(i32, u32, u32)> = (0..15)
        .map(|i| {
            JulianDay(find_shuo(EPOCH_SHUO_JD + (shuo_idx + i) as f64 * SYNODIC_MONTH) + UTC8)
                .to_ymd()
        })
        .collect();

    let leap = if moons.len() > 13 {
        (1..moons.len().saturating_sub(1))
            .find(|&i| find_mid_qi(&moons[i], &moons[i + 1]).is_none())
    } else {
        None
    };

    let idx = month_to_index(lunar.month, leap, lunar.is_leap, lunar.year, anchor_year);
    if idx >= moons.len() {
        return (lunar.year, lunar.month, lunar.day);
    }

    // 逆向 solar_to_lunar：新月日历日 + (day - 1) 天 = 目标公历日
    let (nm_y, nm_m, nm_d) = moons[idx];
    let jd_nm = JulianDay::from_ymd(nm_y, nm_m, nm_d).0;
    let target_jd = jd_nm + (lunar.day as f64 - 1.0);
    JulianDay(target_jd).to_ymd()
}

/// 近似第 n 个朔日 JD
///
/// `jd` 为参考日（通常用纪元朔日），`n` 为偏移月数。
/// 返回平朔近似值，需经 `find_shuo`（或 `calc_shuo`）迭代精化。
pub fn shuo(jd: f64, n: i32) -> f64 {
    jd + n as f64 * SYNODIC_MONTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_2024() {
        let l = solar_to_lunar(2024, 2, 10);
        assert!(!l.is_leap);
    }

    #[test]
    fn test_lunar_roundtrip() {
        let case = (2024, 2, 10);
        let lunar = solar_to_lunar(case.0, case.1, case.2);
        // UTC+8 对齐后正月初一正确回到 2 月 10 日
        assert_eq!(lunar.year, 2024);
        assert_eq!(lunar.month, 1);
        assert_eq!(lunar.day, 1);
        // 往返正确性
        let (ry, rm, rd) = lunar_to_solar(&lunar);
        assert_eq!((ry, rm, rd), case);
    }

    /// 已知农历日期验证（UTC+8 日界对齐后全部正确）
    #[test]
    fn test_known_dates() {
        let cases = [
            (2024, 1, 18, 2023, 12, 8, false, "腊八"),
            (2024, 2, 10, 2024, 1, 1, false, "春节"),
            (2024, 6, 10, 2024, 5, 5, false, "端午"),
            (2024, 9, 17, 2024, 8, 15, false, "中秋"),
            (2024, 10, 11, 2024, 9, 9, false, "重阳"),
        ];
        for &(y, m, d, ly, lm, ld, ll, lab) in &cases {
            let lunar = solar_to_lunar(y, m, d);
            assert_eq!(lunar.year, ly, "{}: year", lab);
            assert_eq!(lunar.month, lm, "{}: month (xcal={})", lab, lunar.month);
            assert_eq!(lunar.day, ld, "{}: day (xcal={})", lab, lunar.day);
            assert_eq!(lunar.is_leap, ll, "{}: leap", lab);
        }
    }
}
