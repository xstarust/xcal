//! 公历工具

use crate::time::jd::JulianDay;

/// 公历日期
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolarDate {
    /// 公历年份
    pub year: i32,
    /// 公历月份 (1-12)
    pub month: u32,
    /// 公历日期 (1-31)
    pub day: u32,
}

impl SolarDate {
    /// 创建公历日期
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// 转换为儒略日
    pub fn to_julian_day(&self) -> JulianDay {
        JulianDay::from_ymd(self.year, self.month, self.day)
    }

    /// 从儒略日创建
    pub fn from_julian_day(jd: JulianDay) -> Self {
        let (y, m, d) = jd.to_ymd();
        Self::new(y, m, d)
    }
}

/// 判断是否为闰年
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// 获取月份天数
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000)); // 能被 400 整除
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900)); // 能被 100 但不能被 400 整除
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 2), 29); // 闰年
        assert_eq!(days_in_month(2023, 2), 28); // 平年
        assert_eq!(days_in_month(2024, 4), 30);
        assert_eq!(days_in_month(2024, 13), 0); // 非法月份
    }

    #[test]
    fn test_solar_date_jd_roundtrip() {
        let d = SolarDate::new(2024, 8, 16);
        let jd = d.to_julian_day();
        let back = SolarDate::from_julian_day(jd);
        assert_eq!(d, back);
    }

    #[test]
    fn test_solar_date_epoch() {
        let d = SolarDate::new(2000, 1, 1);
        let jd = d.to_julian_day();
        assert!(jd.0 - 2451545.0 < 0.001, "J2000 JD: {}", jd.0);
    }
}
