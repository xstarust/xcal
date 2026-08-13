//! 节气计算
//!
//! 基于太阳黄经的牛顿迭代法精确计算节气发生时刻。
//!
//! 节气黄经角度：`((索引 + 21) % 24) * 15°`
//!   立春=315°, 春分=0°, 夏至=90°, 秋分=180°, 冬至=270°

use crate::core::eph0::sun_longitude_deg;
use crate::time::jd::JulianDay;

/// 二十四节气枚举（自 立春 开始）
///
/// 每个变体的 `usize` 值即为其索引：立春=0, 雨水=1, ..., 大寒=23。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Jieqi {
    Lichun,      // 0 立春
    Yushui,      // 1 雨水
    Jingzhe,     // 2 惊蛰
    Chunfen,     // 3 春分
    Qingming,    // 4 清明
    Guyu,        // 5 谷雨
    Lixia,       // 6 立夏
    Xiaoman,     // 7 小满
    Mangzhong,   // 8 芒种
    Xiazhi,      // 9 夏至
    Xiaoshu,     // 10 小暑
    Dashu,       // 11 大暑
    Liqiu,       // 12 立秋
    Chushu,      // 13 处暑
    Bailu,       // 14 白露
    Qiufen,      // 15 秋分
    Hanlu,       // 16 寒露
    Shuangjiang, // 17 霜降
    Lidong,      // 18 立冬
    Xiaoxue,     // 19 小雪
    Daxue,       // 20 大雪
    Dongzhi,     // 21 冬至
    Xiaohan,     // 22 小寒
    Dahan,       // 23 大寒
}

impl Jieqi {
    /// 全部 24 节气（有序）
    pub const ALL: [Self; 24] = [
        Self::Lichun,
        Self::Yushui,
        Self::Jingzhe,
        Self::Chunfen,
        Self::Qingming,
        Self::Guyu,
        Self::Lixia,
        Self::Xiaoman,
        Self::Mangzhong,
        Self::Xiazhi,
        Self::Xiaoshu,
        Self::Dashu,
        Self::Liqiu,
        Self::Chushu,
        Self::Bailu,
        Self::Qiufen,
        Self::Hanlu,
        Self::Shuangjiang,
        Self::Lidong,
        Self::Xiaoxue,
        Self::Daxue,
        Self::Dongzhi,
        Self::Xiaohan,
        Self::Dahan,
    ];

    /// 节气名称（中文）
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lichun => "立春",
            Self::Yushui => "雨水",
            Self::Jingzhe => "惊蛰",
            Self::Chunfen => "春分",
            Self::Qingming => "清明",
            Self::Guyu => "谷雨",
            Self::Lixia => "立夏",
            Self::Xiaoman => "小满",
            Self::Mangzhong => "芒种",
            Self::Xiazhi => "夏至",
            Self::Xiaoshu => "小暑",
            Self::Dashu => "大暑",
            Self::Liqiu => "立秋",
            Self::Chushu => "处暑",
            Self::Bailu => "白露",
            Self::Qiufen => "秋分",
            Self::Hanlu => "寒露",
            Self::Shuangjiang => "霜降",
            Self::Lidong => "立冬",
            Self::Xiaoxue => "小雪",
            Self::Daxue => "大雪",
            Self::Dongzhi => "冬至",
            Self::Xiaohan => "小寒",
            Self::Dahan => "大寒",
        }
    }

    /// 节气对应的太阳黄经（度）
    ///
    /// 立春=315°, 春分=0°, 夏至=90°, 秋分=180°, 冬至=270°
    pub fn angle(self) -> f64 {
        ((self as usize + 21) % 24) as f64 * 15.0
    }

    /// 计算指定年份该节气的精确 JD
    pub fn jd(self, year: i32) -> f64 {
        let angle = self.angle();
        // 初始近似：该节气大约在立春 + n * 15.2 天
        let lichun_approx = JulianDay::from_ymd(year, 2, 4).0;
        let days_offset = ((angle + 360.0 - 315.0) % 360.0) / 0.9856;
        let approx_jd = lichun_approx + days_offset;
        find_jieqi_jd(approx_jd, angle)
    }

    /// 指定年份该节气的公历日期 `(年, 月, 日)`
    pub fn date(self, year: i32) -> (i32, u32, u32) {
        JulianDay(self.jd(year)).to_ymd()
    }
}

impl From<usize> for Jieqi {
    fn from(idx: usize) -> Self {
        Self::ALL[idx % 24]
    }
}

/// 用牛顿迭代法找太阳到达指定黄经的精确时刻
fn find_jieqi_jd(jd_approx: f64, target_lon: f64) -> f64 {
    let mut jd = jd_approx;
    for _ in 0..12 {
        let lon = sun_longitude_deg(jd);
        let mut diff = (lon - target_lon).rem_euclid(360.0);
        if diff > 180.0 {
            diff -= 360.0;
        }
        if diff.abs() < 1e-6 {
            break;
        }
        jd -= diff * 365.25 / 360.0;
    }
    jd
}

/// 获取指定年份的第 n 个节气的 JD
///
/// 保留函数形式，与 `Jieqi::from(n).jd(year)` 等价。
pub fn jieqi(year: i32, n: u32) -> f64 {
    Jieqi::from(n as usize).jd(year)
}

/// 获取指定年份的全年节气列表
pub fn jieqi_list(year: i32) -> Vec<(Jieqi, f64)> {
    Jieqi::ALL.iter().map(|&j| (j, j.jd(year))).collect()
}

/// 获取立春 JD
pub fn lichun_jd(year: i32) -> f64 {
    Jieqi::Lichun.jd(year)
}

/// 判断某 JD 是否在立春之前
pub fn is_before_lichun(jd: f64, year: i32) -> bool {
    jd < lichun_jd(year)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jieqi_angle() {
        assert_eq!(Jieqi::Lichun.angle(), 315.0);
        assert_eq!(Jieqi::Chunfen.angle(), 0.0);
        assert_eq!(Jieqi::Xiazhi.angle(), 90.0);
        assert_eq!(Jieqi::Qiufen.angle(), 180.0);
        assert_eq!(Jieqi::Dongzhi.angle(), 270.0);
    }

    #[test]
    fn test_jieqi_angle_wraps() {
        // Dahan = index 23 -> (23+21)%24 = 20 -> 20*15 = 300°
        assert_eq!((Jieqi::Dahan as usize + 21) % 24, 20);
        assert_eq!(Jieqi::Dahan.angle(), 300.0);
    }

    #[test]
    fn test_jieqi_name() {
        assert_eq!(Jieqi::Lichun.name(), "立春");
        assert_eq!(Jieqi::Xiazhi.name(), "夏至");
        assert_eq!(Jieqi::Dongzhi.name(), "冬至");
    }

    #[test]
    fn test_lichun_jd_2024() {
        let jd = lichun_jd(2024);
        let (y, m, d) = JulianDay(jd).to_ymd();
        assert_eq!(y, 2024);
        assert_eq!(m, 2);
        // 立春在 2/3~2/5
        assert!((3..=5).contains(&d), "lichun 2024 day: {}", d);
    }

    #[test]
    fn test_is_before_lichun() {
        let before = JulianDay::from_ymd(2024, 2, 1).0;
        assert!(is_before_lichun(before, 2024));
        let after = JulianDay::from_ymd(2024, 3, 1).0;
        assert!(!is_before_lichun(after, 2024));
    }

    #[test]
    fn test_jieqi_list_2024_len() {
        let list = jieqi_list(2024);
        assert_eq!(list.len(), 24);
    }

    #[test]
    fn test_jieqi_list_ordered() {
        let list = jieqi_list(2024);
        for i in 0..23 {
            assert!(
                list[i].1 < list[i + 1].1,
                "{} should come before {}",
                list[i].0.name(),
                list[i + 1].0.name()
            );
        }
    }
}
