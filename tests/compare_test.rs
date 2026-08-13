//! xcal 农历对比测试
//!
//! 基于 UTC+8 日界对齐（农历以北京时间午夜为日界）。

use xcal::calendar::lunar::{LunarDate, lunar_to_solar, solar_to_lunar};

#[test]
fn test_lunar_roundtrip_consistency() {
    let dates = [
        (2024, 2, 10),
        (2024, 2, 9),
        (2024, 6, 10),
        (2024, 9, 17),
        (2024, 10, 11),
        (2024, 1, 18),
        (2023, 1, 22),
        (2023, 2, 20),
        (2023, 3, 22),
        (2023, 4, 20),
        (2025, 1, 29),
        (2023, 1, 1),
        (2023, 12, 1),
        (2024, 1, 1),
        (2024, 6, 1),
        (2024, 9, 1),
        (2025, 6, 1),
        (2025, 12, 1),
        (2020, 5, 1),
        (2019, 7, 1),
        (2019, 12, 31),
    ];
    for &(y, m, d) in &dates {
        let lunar = solar_to_lunar(y, m, d);
        let (ry, rm, rd) = lunar_to_solar(&lunar);
        assert_eq!(
            (ry, rm, rd),
            (y, m, d),
            "roundtrip: {}-{:02}-{:02} → lunar {}-{:02}-{:02} leap={} → {}-{:02}-{:02}",
            y,
            m,
            d,
            lunar.year,
            lunar.month,
            lunar.day,
            lunar.is_leap,
            ry,
            rm,
            rd
        );
    }
}

#[test]
fn test_leap_month_2023() {
    let dates = [
        (2023, 1, 22, 2023, 1, 1, false, "正月初一"),
        (2023, 2, 20, 2023, 2, 1, false, "二月初一"),
        (2023, 3, 22, 2023, 2, 1, true, "闰二月初一"),
        (2023, 4, 20, 2023, 3, 1, false, "三月初一"),
    ];
    for &(y, m, d, ly, lm, ld, ll, label) in &dates {
        let lunar = solar_to_lunar(y, m, d);
        assert_eq!(
            (lunar.year, lunar.month, lunar.day, lunar.is_leap),
            (ly, lm, ld, ll),
            "{}: got ({}-{:02}-{:02} leap={})",
            label,
            lunar.year,
            lunar.month,
            lunar.day,
            lunar.is_leap
        );
    }
}

#[test]
fn test_lunar_to_solar_known() {
    let cases = [
        (2024, 1, 1, false, 2024, 2, 10, "2024初一"),
        (2023, 1, 1, false, 2023, 1, 22, "2023初一"),
        (2023, 2, 1, false, 2023, 2, 20, "2023二月初一"),
        (2023, 2, 1, true, 2023, 3, 22, "2023闰二月初一"),
    ];
    for &(ly, lm, ld, ll, y, m, d, label) in &cases {
        let lunar = LunarDate::new(ly, lm, ld, ll);
        let (ry, rm, rd) = lunar_to_solar(&lunar);
        assert_eq!(
            (ry, rm, rd),
            (y, m, d),
            "{}: got {}-{:02}-{:02}",
            label,
            ry,
            rm,
            rd
        );
    }
}
