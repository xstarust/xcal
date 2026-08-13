//! 儒略日计算

use crate::CalxError;

/// 儒略日
///
/// 标准天文学儒略日，J2000.0 = 2451545.0（对应 2000-01-01 12:00 TT）
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct JulianDay(pub f64);

impl JulianDay {
    /// 从公历日期创建儒略日（UT）
    ///
    /// 1582-10-15 之后的日期使用格里高利历公式，
    /// 之前的日期使用儒略历公式。
    ///
    /// # Arguments
    /// * `year` - 年份
    /// * `month` - 月份 (1-12)
    /// * `day` - 日期 (1-31)
    ///
    /// # Examples
    ///
    /// ```
    /// use xcal::time::jd::JulianDay;
    /// let jd = JulianDay::from_ymd(2000, 1, 1);
    /// assert!((jd.0 - 2451544.5).abs() < 0.01);
    /// ```
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        let (y, m) = if month <= 2 {
            (year - 1, month as i32 + 12)
        } else {
            (year, month as i32)
        };

        // 使用公历公式之前检查是否需要在儒略历和公历之间切换。
        // 格里高利历从 1582-10-15 开始生效
        let (y, m) = (y as f64, m as f64);
        let jd = if year > 1582 || (year == 1582 && (month > 10 || (month == 10 && day >= 15))) {
            // 格里高利历公式
            let a = (y / 100.0).floor();
            let b = 2.0 - a + (a / 4.0).floor();
            (365.25 * (y + 4716.0)).floor() + (30.6001 * (m + 1.0)).floor() + day as f64 + b
                - 1524.5
        } else {
            // 儒略历公式（无格里高利校正）
            (365.25 * (y + 4716.0)).floor() + (30.6001 * (m + 1.0)).floor() + day as f64 - 1524.5
        };
        JulianDay(jd)
    }

    /// 转换为公历日期 (年, 月, 日)
    ///
    /// JD < 2299161 的日期使用儒略历算法
    pub fn to_ymd(&self) -> (i32, u32, u32) {
        let jd = self.0 + 0.5;
        let z = jd.floor() as i64;
        let a = if z < 2299161 {
            z
        } else {
            let alpha = ((z as f64 - 1867216.25) / 36524.25).floor() as i64;
            z + 1 + alpha - (alpha / 4)
        };
        let b = a + 1524;
        let c = ((b as f64 - 122.1) / 365.25).floor() as i64;
        let d = (365.25 * c as f64).floor() as i64;
        let e = ((b as f64 - d as f64) / 30.6001).floor() as i64;
        let day = (b - d - (30.6001 * e as f64).floor() as i64) as u32;
        let month = if e < 14 { e - 1 } else { e - 13 } as u32;
        let year = if month > 2 { c - 4716 } else { c - 4715 } as i32;
        (year, month, day)
    }

    /// 创建带验证的儒略日
    pub fn new(value: f64) -> Result<Self, CalxError> {
        if !value.is_finite() {
            return Err(CalxError::Other("儒略日必须为有限数值".into()));
        }
        Ok(JulianDay(value))
    }

    /// 获取儒略日浮点值
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl std::fmt::Display for JulianDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}", self.0)
    }
}

/// 计算两个儒略日之间的天数差
pub fn julian_diff(jd1: JulianDay, jd2: JulianDay) -> f64 {
    jd2.0 - jd1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_j2000() {
        // 2000-01-01 00:00 UT = JD 2451544.5
        let jd = JulianDay::from_ymd(2000, 1, 1);
        assert!(
            (jd.0 - 2451544.5).abs() < 0.01,
            "expected 2451544.5, got {}",
            jd.0
        );
    }

    #[test]
    fn test_to_ymd_roundtrip() {
        let test_dates: &[(i32, u32, u32)] = &[
            (2000, 1, 1),
            (2024, 6, 21),
            (1999, 12, 31),
            (2020, 2, 29),
            (1900, 1, 1),
            (1582, 10, 15),
            (1582, 10, 4), // 儒略历日期
            (1000, 1, 1),
            (1, 1, 1),
        ];
        for &(y, m, d) in test_dates {
            let jd = JulianDay::from_ymd(y, m, d);
            let (y2, m2, d2) = jd.to_ymd();
            assert_eq!(
                (y, m, d),
                (y2, m2, d2),
                "Roundtrip failed for {}-{}-{}: got {}-{}-{}",
                y,
                m,
                d,
                y2,
                m2,
                d2
            );
        }
    }

    #[test]
    fn test_jd_new_validates() {
        assert!(JulianDay::new(2451545.0).is_ok());
        assert!(JulianDay::new(f64::INFINITY).is_err());
        assert!(JulianDay::new(f64::NAN).is_err());
    }

    #[test]
    fn test_display() {
        let jd = JulianDay(2451545.0);
        assert_eq!(jd.to_string(), "2451545.0");
    }
}
