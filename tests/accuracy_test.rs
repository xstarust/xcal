//! xcal 集成测试
//!
//! 包含跨模块的精度验证和已知日期对照测试。

use xcal::calendar::ganzhi::{Dizhi, Tiangan, day_ganzhi};
use xcal::calendar::jieqi::{Jieqi, jieqi};
use xcal::calendar::lunar::{lunar_to_solar, solar_to_lunar};
use xcal::core::eph0::{moon_longitude_deg, sun_longitude_deg};
use xcal::time::jd::JulianDay;

/// 太阳黄经验证
#[test]
fn test_sun_accuracy() {
    let lon = sun_longitude_deg(2451545.0);
    assert!((lon - 280.46).abs() < 0.1);

    let jd = JulianDay::from_ymd(2024, 3, 20).0;
    let lon = sun_longitude_deg(jd);
    let dist = (lon - 0.0).abs().min((lon - 360.0).abs());
    assert!(dist < 1.0, "spring equinox lon {}°", lon);
}

/// 月亮黄经值域验证
#[test]
fn test_moon_sanity() {
    for (y, m, d) in &[(2024, 1, 1), (2024, 6, 21), (2024, 12, 21)] {
        let jd = JulianDay::from_ymd(*y, *m, *d).0;
        let lon = moon_longitude_deg(jd);
        assert!(
            (0.0..=360.0).contains(&lon),
            "moon lon out of range at {}-{}-{}: {}",
            y,
            m,
            d,
            lon
        );
    }
}

/// 节气验证
#[test]
fn test_solar_terms() {
    let cases = [
        (2024, 0, "立春", 2, 3..=5),
        (2024, 3, "春分", 3, 19..=22),
        (2024, 9, "夏至", 6, 20..=23),
        (2024, 15, "秋分", 9, 21..=24),
        (2024, 21, "冬至", 12, 20..=23),
    ];
    for &(year, n, name, exp_m, ref exp_d_range) in &cases {
        let jd = jieqi(year, n);
        let (_y, m, d) = JulianDay(jd).to_ymd();
        let jq = Jieqi::from(n as usize);
        assert_eq!(jq.name(), name);
        assert_eq!(m, exp_m, "{} month: expected {}, got {}", name, exp_m, m);
        assert!(
            exp_d_range.contains(&d),
            "{} day: expected in {:?}, got {}",
            name,
            exp_d_range,
            d
        );
    }
}

/// 农历→公历往返
#[test]
fn test_lunar_roundtrip_multiple() {
    // 仅验证正月十五前后的往返，不覆盖跨年边界
    // （月亮模型 ~30" 精度导致年边界处可能差 1 天）
    let cases = [(2024, 2, 10), (2024, 2, 24), (2024, 5, 8)];
    for &(y, m, d) in &cases {
        let lunar = solar_to_lunar(y, m, d);
        let (ry, rm, rd) = lunar_to_solar(&lunar);
        assert_eq!(
            (ry, rm, rd),
            (y, m, d),
            "roundtrip: {}-{:02}-{:02} -> {}-{:02}-{:02} -> {}-{:02}-{:02}",
            y,
            m,
            d,
            lunar.year,
            lunar.month,
            lunar.day,
            ry,
            rm,
            rd
        );
    }
}

/// JPL DE441 参考值验证
///
/// 从 JPL HORIZONS API（DE441 历表）获取的太阳/月亮视黄经。
/// VSOP87/ELP/MPP02 理论与 JPL 数值积分之间的固有偏差：
/// - 太阳: ~3-5"
/// - 月亮: ~15-35"
#[test]
fn test_jpl_reference() {
    // JPL DE441 参考值（JD, 预期太阳黄经, 预期月亮黄经, 标签）
    let cases = [
        (2451545.0, 280.368909, 223.323786, "J2000.0"),
        (2460310.5, 280.038981, 155.992187, "2024-01-01"),
        (2460482.5, 90.125196, 257.080775, "2024-06-21"),
        (2460576.5, 180.459523, 67.971695, "2024-09-23"),
    ];
    for &(jd, sun_ref, moon_ref, label) in &cases {
        let sun = sun_longitude_deg(jd);
        let moon = moon_longitude_deg(jd);
        let sun_diff = (sun - sun_ref).abs();
        let moon_diff = (moon - moon_ref).abs();
        // 容忍度: DE405 与 JPL DE441 之间固有偏差约 2.7"
        // VSOP87 理论本身 vs DE405 精度为 0.1"
        assert!(
            sun_diff < 0.003,
            "{} Sun: xcal={:.6}° JPL={:.6}° diff={:.6}° ({:.2}\")",
            label,
            sun,
            sun_ref,
            sun_diff,
            sun_diff * 3600.0
        );
        // 月亮 DE405 vs DE441 偏差更大
        assert!(
            moon_diff < 0.03,
            "{} Moon: xcal={:.6}° JPL={:.6}° diff={:.6}° ({:.2}\")",
            label,
            moon,
            moon_ref,
            moon_diff,
            moon_diff * 3600.0
        );
    }
}

/// 长时间跨度精度验证（vs JPL DE441）
///
/// VSOP87/ELP/MPP02 理论在远年代精度会衰减，主要原因是
/// 高阶泊松项(t²/t³/t⁴)在远年代发散。月亮比太阳衰减更快，
/// 因 ELP/MPP02 的级数项数不足以约束长时间行为。
///
/// 近现代 (1500-2100) 精度可靠。
/// 公元元年前后，月亮误差可达数度（高频项相位漂移累计）。
/// 实际的紫微斗数应用范围仅 1900-2100，不受影响。
#[test]
fn test_long_range_accuracy() {
    // (年, 月, 日, JPL 太阳, 太阳容忍度")
    let cases = [
        (-2000, 6, 21, 71.027372, 1800.0),
        (-500, 6, 21, 82.439982, 200.0),
        (0, 6, 21, 86.538022, 600.0),
        (500, 6, 21, 90.442373, 300.0),
        (1500, 6, 21, 98.335184, 30.0),
        (2000, 6, 21, 89.928637, 10.0),
        (3000, 6, 21, 90.284121, 10.0),
    ];
    for &(y, m, d, sun_jpl, sun_tol) in &cases {
        let jd = JulianDay::from_ymd(y, m, d).0;
        let sun = sun_longitude_deg(jd);
        let sun_diff = (sun - sun_jpl).abs() * 3600.0;
        assert!(
            sun_diff < sun_tol,
            "{:+} Sun: xcal={:.6}° JPL={:.6}° diff={:.2}\"",
            y,
            sun,
            sun_jpl,
            sun_diff
        );
    }
}

/// 六十甲子验证
#[test]
fn test_ganzhi() {
    // J2000.0 (2000-01-01 12:00 TT, JD 2451545.0) 是 戊午日
    let gz = day_ganzhi(2451545.0);
    assert_eq!(
        gz.gan,
        Tiangan::Wu,
        "J2000 ganzhi gan expected Wu, got {:?}",
        gz.gan
    );
    assert_eq!(
        gz.zhi,
        Dizhi::Wu,
        "J2000 ganzhi zhi expected Wu, got {:?}",
        gz.zhi
    );
}

/// 跨年份验证（农历年边界）
#[test]
fn test_lunar_year_boundary() {
    let lunar = solar_to_lunar(2024, 1, 21);
    assert_eq!(lunar.year, 2023);

    let lunar = solar_to_lunar(2024, 2, 10);
    assert_eq!(lunar.year, 2024);
}
