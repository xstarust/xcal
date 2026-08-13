//! 地理位置

use crate::time::solar_time::{calc_time_correction, calc_utc_offset};

/// 地理位置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    /// 经度（东经为正，西经为负）
    pub longitude: f64,
    /// 纬度（北纬为正，南纬为负）
    pub latitude: f64,
}

impl Location {
    /// 创建地理位置
    pub fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    /// 根据经度推算时区偏移（小时）
    pub fn calc_utc_offset(&self) -> f64 {
        calc_utc_offset(self.longitude)
    }

    /// 计算真太阳时校正量（秒）
    pub fn calc_time_correction(&self, tz_offset: f64) -> i64 {
        calc_time_correction(self.longitude, tz_offset)
    }
}

impl std::hash::Hash for Location {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.longitude.to_bits().hash(state);
        self.latitude.to_bits().hash(state);
    }
}
