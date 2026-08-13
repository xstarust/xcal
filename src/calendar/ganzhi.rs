//! 六十甲子计算

use crate::calendar::jieqi::{is_before_lichun, jieqi};
use crate::time::jd::JulianDay;
use crate::time::solar_time::equation_of_time_seconds;

/// 天干
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tiangan {
    Jia,
    Yi,
    Bing,
    Ding,
    Wu,
    Ji,
    Geng,
    Xin,
    Ren,
    Gui,
}

impl Tiangan {
    pub fn from_index(index: usize) -> Self {
        match index % 10 {
            0 => Tiangan::Jia,
            1 => Tiangan::Yi,
            2 => Tiangan::Bing,
            3 => Tiangan::Ding,
            4 => Tiangan::Wu,
            5 => Tiangan::Ji,
            6 => Tiangan::Geng,
            7 => Tiangan::Xin,
            8 => Tiangan::Ren,
            9 => Tiangan::Gui,
            _ => unreachable!(),
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tiangan::Jia => 0,
            Tiangan::Yi => 1,
            Tiangan::Bing => 2,
            Tiangan::Ding => 3,
            Tiangan::Wu => 4,
            Tiangan::Ji => 5,
            Tiangan::Geng => 6,
            Tiangan::Xin => 7,
            Tiangan::Ren => 8,
            Tiangan::Gui => 9,
        }
    }
}

/// 地支
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dizhi {
    Zi,
    Chou,
    Yin,
    Mao,
    Chen,
    Si,
    Wu,
    Wei,
    Shen,
    You,
    Xu,
    Hai,
}

impl Dizhi {
    pub fn from_index(index: usize) -> Self {
        match index % 12 {
            0 => Dizhi::Zi,
            1 => Dizhi::Chou,
            2 => Dizhi::Yin,
            3 => Dizhi::Mao,
            4 => Dizhi::Chen,
            5 => Dizhi::Si,
            6 => Dizhi::Wu,
            7 => Dizhi::Wei,
            8 => Dizhi::Shen,
            9 => Dizhi::You,
            10 => Dizhi::Xu,
            11 => Dizhi::Hai,
            _ => unreachable!(),
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Dizhi::Zi => 0,
            Dizhi::Chou => 1,
            Dizhi::Yin => 2,
            Dizhi::Mao => 3,
            Dizhi::Chen => 4,
            Dizhi::Si => 5,
            Dizhi::Wu => 6,
            Dizhi::Wei => 7,
            Dizhi::Shen => 8,
            Dizhi::You => 9,
            Dizhi::Xu => 10,
            Dizhi::Hai => 11,
        }
    }
}

/// 干支
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GanZhi {
    pub gan: Tiangan,
    pub zhi: Dizhi,
}

/// 四柱八字
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bazi {
    /// 年柱
    pub year: GanZhi,
    /// 月柱
    pub month: GanZhi,
    /// 日柱
    pub day: GanZhi,
    /// 时柱
    pub hour: GanZhi,
}

/// 计算完整四柱八字
///
/// 输入**标准时**（如北京时间 12:00），输出年/月/日/时四柱。
/// 年柱以立春为界（天文标准），月柱以节气为界。
///
/// 如需以真太阳时为基准，先调用 `convert_to_apparent_solar_time` 校正时间后传入。
///
/// # Arguments
/// - `year` / `month` / `day` — 公历日期
/// - `hour` / `minute` — 标准时（0~23, 0~59）
///
/// 八字计算（内部自动做 LMT + EOT 真太阳时校正）
///
/// 接口输入**标准时**（如北京时间 12:00）+ **经度**，内部通过 xcal 的
/// 真太阳时计算校正后推导四柱八字。下层的 `day_ganzhi` / `month_ganzhi` 等
/// 函数仍可用标准时，此函数是"一站式"入口。
pub fn bazi(year: i32, month: u32, day: u32, hour: u32, minute: u32, longitude: f64) -> Bazi {
    // 标准时 → 真太阳时 JD
    let jd_midnight = JulianDay::from_ymd(year, month, day).0;
    let jd_std = jd_midnight + hour as f64 / 24.0 + minute as f64 / 1440.0;
    let mlon = (longitude / 15.0).round() * 15.0;
    let jd_lmt = jd_std + (longitude - mlon) / 15.0 / 24.0;
    let jd_apparent = jd_lmt + equation_of_time_seconds(jd_lmt) / 86400.0;

    // 真太阳时午夜正午
    let jd_app_mid = jd_apparent.floor();
    let jd_app_noon = jd_app_mid + 0.5;

    // 时辰索引（标准时的输入决定，不因 EOT 变）
    let hour_index = ((hour as i32 + 1) / 2) as usize % 12;

    // 日柱、时柱、年柱、月柱全在真太阳时 JD 上计算
    let day_gz = day_ganzhi(jd_app_noon);
    let hour_gz = GanZhi {
        gan: Tiangan::from_index(rat_rule(day_gz.gan).index() + hour_index),
        zhi: Dizhi::from_index(hour_index),
    };
    let (ay, _, _) = JulianDay(jd_apparent).to_ymd();
    let year_gz = year_ganzhi(ay, is_before_lichun(jd_apparent, ay));
    let month_gz = month_ganzhi(year_gz.gan, jd_apparent);

    Bazi {
        year: year_gz,
        month: month_gz,
        day: day_gz,
        hour: hour_gz,
    }
}

/// 日柱干支
///
/// 基于儒略日计算日柱。公式：`(floor(jd + 0.5) - 11) % 60`，
/// 其中常量 11 来自校准：2000-01-01 midnight（JD 2451544.5, day_num=2451545）为丁午日(idx=54)。
pub fn day_ganzhi(jd: f64) -> GanZhi {
    let day_num = (jd + 0.5).floor() as i64;
    let idx = ((day_num - 11) % 60 + 60) % 60;
    GanZhi {
        gan: Tiangan::from_index(idx as usize),
        zhi: Dizhi::from_index(idx as usize),
    }
}

/// 年柱干支（以立春为界）
pub fn year_ganzhi(year: i32, is_before_lichun: bool) -> GanZhi {
    let base_year = 1984; // 甲子年
    let y = if is_before_lichun { year - 1 } else { year };
    let idx = ((y - base_year) % 60 + 60) % 60;
    GanZhi {
        gan: Tiangan::from_index(idx as usize),
        zhi: Dizhi::from_index(idx as usize),
    }
}

/// 五虎遁（年干 → 正月寅月天干）
///
/// 口诀: "甲己之年丙作首，乙庚之岁戊为头，
///        丙辛之年从庚起，丁壬壬位顺行流，
///        戊癸甲寅之上求。"
fn tiger_rule(year_tg: Tiangan) -> Tiangan {
    match year_tg {
        Tiangan::Jia | Tiangan::Ji => Tiangan::Bing,
        Tiangan::Yi | Tiangan::Geng => Tiangan::Wu,
        Tiangan::Bing | Tiangan::Xin => Tiangan::Geng,
        Tiangan::Ding | Tiangan::Ren => Tiangan::Ren,
        Tiangan::Wu | Tiangan::Gui => Tiangan::Jia,
    }
}

/// 五鼠遁（日干 → 子时天干）
///
/// 口诀: "甲己还加甲，乙庚丙作初，
///        丙辛从戊起，丁壬庚子居，
///        戊癸何方发，壬子是真途。"
fn rat_rule(day_tg: Tiangan) -> Tiangan {
    match day_tg {
        Tiangan::Jia | Tiangan::Ji => Tiangan::Jia,
        Tiangan::Yi | Tiangan::Geng => Tiangan::Bing,
        Tiangan::Bing | Tiangan::Xin => Tiangan::Wu,
        Tiangan::Ding | Tiangan::Ren => Tiangan::Geng,
        Tiangan::Wu | Tiangan::Gui => Tiangan::Ren,
    }
}

/// 月支：通过节气判断 JD 所在月的月地支
///
/// 节气月以节（立春/惊蛰/清明...）为分界：
///   寅月(立春~惊蛰前), 卯月(惊蛰~清明前), ..., 丑月(小寒~立春前)
pub fn month_branch(jd: f64) -> Dizhi {
    // 12 个节在 JIEQI_NAMES 中的索引及对应的月地支：
    //   立春(n=0)→寅(2), 惊蛰(n=2)→卯(3), 清明(n=4)→辰(4),
    //   立夏(n=6)→巳(5), 芒种(n=8)→午(6), 小暑(n=10)→未(7),
    //   立秋(n=12)→申(8), 白露(n=14)→酉(9), 寒露(n=16)→戌(10),
    //   立冬(n=18)→亥(11), 大雪(n=20)→子(0), 小寒(n=22)→丑(1)
    const MONTH_STARTS: [(u32, usize); 12] = [
        (0, 2),
        (2, 3),
        (4, 4),
        (6, 5),
        (8, 6),
        (10, 7),
        (12, 8),
        (14, 9),
        (16, 10),
        (18, 11),
        (20, 0),
        (22, 1),
    ];

    let (year, _, _) = JulianDay(jd).to_ymd();
    let spring_this = jieqi(year, 0);
    let spring_prev = jieqi(year - 1, 0);

    // 确定 JD 所属的节气年：以立春为界
    // 节气年从立春(y)到立春(y+1)
    let solar_term_year = if jd >= spring_this {
        year
    } else if jd >= spring_prev {
        year - 1
    } else {
        year - 2
    };

    // 计算该节气年的 12 个月节 JD
    // 注意：小寒(n=22) 出现在下一自然年，属节气年的末尾
    let mut boundaries: Vec<(f64, usize)> = MONTH_STARTS
        .iter()
        .map(|&(n, dz)| {
            // 所有节气索引都是相对于 solar_term_year 立春开始的周期：
            //   - n=0(立春)~n=21 在 solar_term_year 年内或稍后
            //   - n=22(小寒)~n=23(大寒)在 solar_term_year+1 年初，但仍属该节气年
            // get_jieqi(solar_term_year, 22) 从 solar_term_year 立春向前搜索，给出 solar_term_year+1 年 1 月的小寒
            (jieqi(solar_term_year, n), dz)
        })
        .collect();
    // 加下一年的立春作为终止边界
    boundaries.push((jieqi(solar_term_year + 1, 0), 2));

    for i in 0..boundaries.len() - 1 {
        if jd >= boundaries[i].0 && jd < boundaries[i + 1].0 {
            return Dizhi::from_index(boundaries[i].1);
        }
    }
    Dizhi::from_index(2) // 默认寅月
}

/// 月柱完整干支（以节气为月界）
///
/// 先通过节气确定月地支，再通过年干（五虎遁）确定月天干。
/// 适用于年柱以立春为界的场景。
///
/// # Arguments
/// * `year_tg` - 年柱天干（需已按年柱分界规则校准）
/// * `jd` - 目标儒略日
pub fn month_ganzhi(year_tg: Tiangan, jd: f64) -> GanZhi {
    let dz = month_branch(jd);
    let dz_idx = dz.index();

    // 五虎遁：寅月（dz=2）天干已知，其余顺推
    let start_tg = tiger_rule(year_tg);
    // 寅月=0 offset，卯月=1，...，丑月=11
    let offset = (dz_idx + 12 - 2) % 12;
    let gan = Tiangan::from_index(start_tg.index() + offset);

    GanZhi { gan, zhi: dz }
}

/// 时柱干支（以日干为基准）
///
/// 通过日干（五鼠遁）确定子时天干，其余时辰顺推。
///
/// # Arguments
/// * `day_tg` - 日柱天干
/// * `hour_index` - 时辰索引（0=子时=23:00-00:59, 1=丑时, ..., 11=亥时）
pub fn hour_ganzhi(day_tg: Tiangan, hour_index: usize) -> GanZhi {
    let start_tg = rat_rule(day_tg);
    let gan = Tiangan::from_index(start_tg.index() + hour_index);
    let zhi = Dizhi::from_index(hour_index);
    GanZhi { gan, zhi }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_ganzhi_2000_01_01() {
        // 2000-01-01 noon (JD 2451545.0) = 戊午日
        // 验证：2000-08-16 (JD+228天) 为 丙午日(idx=42)，回推得 idx=(42-228)%60=54=戊午
        let gz = day_ganzhi(2451545.0);
        assert_eq!(gz.gan, Tiangan::Wu);
        assert_eq!(gz.zhi, Dizhi::Wu);
    }

    #[test]
    fn test_year_ganzhi_2000() {
        // 2000 年立春前 = 己卯年, 立春后 = 庚辰年
        let before = year_ganzhi(2000, true);
        assert_eq!(before.gan, Tiangan::Ji);
        assert_eq!(before.zhi, Dizhi::Mao);
        let after = year_ganzhi(2000, false);
        assert_eq!(after.gan, Tiangan::Geng);
        assert_eq!(after.zhi, Dizhi::Chen);
    }

    #[test]
    fn test_tiger_rule() {
        assert_eq!(tiger_rule(Tiangan::Jia), Tiangan::Bing);
        assert_eq!(tiger_rule(Tiangan::Yi), Tiangan::Wu);
        assert_eq!(tiger_rule(Tiangan::Geng), Tiangan::Wu);
    }

    #[test]
    fn test_rat_rule() {
        assert_eq!(rat_rule(Tiangan::Jia), Tiangan::Jia);
        assert_eq!(rat_rule(Tiangan::Yi), Tiangan::Bing);
        assert_eq!(rat_rule(Tiangan::Geng), Tiangan::Bing);
    }

    #[test]
    fn test_hour_ganzhi() {
        // 甲日 子时 = 甲子
        let gz = hour_ganzhi(Tiangan::Jia, 0);
        assert_eq!(gz.gan, Tiangan::Jia);
        assert_eq!(gz.zhi, Dizhi::Zi);
        // 甲日 丑时 = 乙丑
        let gz = hour_ganzhi(Tiangan::Jia, 1);
        assert_eq!(gz.gan, Tiangan::Yi);
        assert_eq!(gz.zhi, Dizhi::Chou);
        // 乙日 子时 = 丙子
        let gz = hour_ganzhi(Tiangan::Yi, 0);
        assert_eq!(gz.gan, Tiangan::Bing);
        assert_eq!(gz.zhi, Dizhi::Zi);
    }

    #[test]
    fn test_month_branch_known() {
        // 2024 立春约 2/4 16:27 UTC，立春前月支=Chou(1)，立春后=Yin(2)
        let winter = JulianDay::from_ymd(2024, 2, 3).0;
        assert_eq!(month_branch(winter), Dizhi::Chou, "立春前一天应为丑月");
        // 02-04 00:00 UT 仍在立春前
        assert_eq!(
            month_branch(winter + 1.0),
            Dizhi::Chou,
            "立春日00:00UT仍在丑月"
        );
        // 02-05 00:00 UT 已在立春后
        let after = JulianDay::from_ymd(2024, 2, 5).0;
        assert_eq!(month_branch(after), Dizhi::Yin, "立春次日应为寅月");
    }

    #[test]
    fn test_month_ganzhi_2024() {
        // 2024 年立春后（2/5），年干=庚辰，寅月=戊寅
        let spring = JulianDay::from_ymd(2024, 2, 5).0;
        let gz = month_ganzhi(Tiangan::Geng, spring);
        assert_eq!(gz.gan, Tiangan::Wu, "庚年寅月=戊寅");
        assert_eq!(gz.zhi, Dizhi::Yin);
    }
}
