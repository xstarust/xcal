//! xcal - 天文历法核心库
//!
//! 提供农历、节气、六十甲子、真太阳时计算
//!
//! # 例子
//!
//! ```
//! use xcal::time::jd::JulianDay;
//! use xcal::solar_to_lunar;
//!
//! // 公历转农历
//! let lunar = solar_to_lunar(2024, 2, 10);
//! assert_eq!((lunar.year, lunar.month, lunar.day), (2024, 1, 1));
//!
//! // 儒略日
//! let jd = JulianDay::from_ymd(2000, 1, 1);
//! assert!((jd.0 - 2451544.5).abs() < 0.01);
//! ```

pub mod calendar;
pub mod core;
pub mod error;
pub mod geo;
pub mod time;

pub use calendar::*;
pub use core::*;
pub use error::*;
pub use geo::*;
pub use time::*;
