//! 天文常数

/// 一天的秒数
pub const SECONDS_PER_DAY: f64 = 86400.0;

/// 一年的天数（平均）
pub const DAYS_PER_YEAR: f64 = 365.25;

/// 一世纪的天数
pub const DAYS_PER_CENTURY: f64 = 36525.0;

/// J2000.0 儒略日
pub const J2000: f64 = 2451545.0;

/// 度转弧度
pub const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

/// 弧度转度
pub const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

/// 太阳平均黄经 (J2000.0)
pub const MEAN_LONGITUDE_SUN_J2000: f64 = 280.460;

/// 太阳平均运动 (度/天)
pub const MEAN_MOTION_SUN: f64 = 0.9856474;

/// 月亮平均运动 (度/天)
pub const MEAN_MOTION_MOON: f64 = 13.176396;

/// 岁动率 (度/世纪)
pub const PRECESSION_RATE: f64 = 50.29;

// 重新导出常用常量
pub use std::f64::consts::PI;
