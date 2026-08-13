//! ΔT 修正（历史时间修正）

const DT_TABLE: &[[f64; 5]] = &[
    [-4000.0, 108371.7, -13036.80, 392.000, 0.0000],
    [-500.0, 17201.0, -627.82, 16.170, -0.3413],
    [-150.0, 12200.6, -346.41, 5.403, -0.1593],
    [150.0, 9113.8, -328.13, -1.647, 0.0377],
    [500.0, 5707.5, -391.41, 0.915, 0.3145],
    [900.0, 2203.4, -283.45, 13.034, -0.1778],
    [1300.0, 490.1, -57.35, 2.085, -0.0072],
    [1600.0, 120.0, -9.81, -1.532, 0.1403],
    [1700.0, 10.2, -0.91, 0.510, -0.0370],
    [1800.0, 13.4, -0.72, 0.202, -0.0193],
    [1830.0, 7.8, -1.81, 0.416, -0.0247],
    [1860.0, 8.3, -0.13, -0.406, 0.0292],
    [1880.0, -5.4, 0.32, -0.183, 0.0173],
    [1900.0, -2.3, 2.06, 0.169, -0.0135],
    [1920.0, 21.2, 1.69, -0.304, 0.0167],
    [1940.0, 24.2, 1.22, -0.064, 0.0031],
    [1960.0, 33.2, 0.51, 0.231, -0.0109],
    [1980.0, 51.0, 1.29, -0.026, 0.0032],
    [2000.0, 63.87, 0.1, 0.0, 0.0],
    [2005.0, 64.7, 0.21, 0.0, 0.0],
    [2012.0, 66.8, 0.22, 0.0, 0.0],
    [2016.0, 68.1024, 0.5456, -0.0542, -0.001172],
    [2020.0, 69.3612, 0.0422, -0.0502, 0.006216],
    [2024.0, 69.1752, -0.0335, -0.0048, 0.000811],
    [2028.0, 69.0206, -0.0275, 0.0055, -0.000014],
    [2032.0, 68.9981, 0.0163, 0.0054, 0.000006],
    [2036.0, 69.1498, 0.0599, 0.0053, 0.000026],
    [2040.0, 69.4751, 0.1035, 0.0051, 0.000046],
    [2044.0, 69.9737, 0.1469, 0.0050, 0.000066],
    [2048.0, 70.6451, 0.1903, 0.0049, 0.000085],
    [2050.0, 71.0457, 0.0, 0.0, 0.0],
];

fn extrapolate(year: f64, acceleration: f64) -> f64 {
    let century = (year - 1820.0) / 100.0;
    -20.0 + acceleration * century * century
}

fn calculate(year: f64) -> f64 {
    let last = DT_TABLE[DT_TABLE.len() - 1];
    if year >= last[0] {
        if year > last[0] + 100.0 {
            return extrapolate(year, 31.0);
        }
        let extrapolated = extrapolate(year, 31.0);
        let correction = extrapolate(last[0], 31.0) - last[1];
        return extrapolated - correction * (last[0] + 100.0 - year) / 100.0;
    }

    let index = DT_TABLE
        .windows(2)
        .position(|window| year < window[1][0] - 1e-9)
        .unwrap_or(DT_TABLE.len() - 2);
    let row = DT_TABLE[index];
    let next_year = DT_TABLE[index + 1][0];
    let t1 = (year - row[0]) / (next_year - row[0]) * 10.0;
    let t2 = t1 * t1;
    let t3 = t2 * t1;
    row[1] + row[2] * t1 + row[3] * t2 + row[4] * t3
}

/// 计算 ΔT 修正值（秒）
///
/// 基于 sxwnl 的历史观测数据和推算公式。输入为绝对儒略日，返回 TD−UT1 秒数。
pub fn delta_t(jd: f64) -> f64 {
    let year = (jd - 2451545.0) / 365.2425 + 2000.0;
    calculate(year)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jd_for_year(year: f64) -> f64 {
        2451545.0 + (year - 2000.0) * 365.2425
    }

    #[test]
    fn test_delta_t_historical() {
        let dt = delta_t(jd_for_year(1600.0));
        assert!((dt - 120.0).abs() < 0.01, "delta_t at 1600: {dt}");
    }

    #[test]
    fn test_delta_t_reference_points() {
        let cases = [
            (2000.0, 63.87),
            (2020.0, 69.3612),
            (2024.0, 69.1752),
            (2028.0, 69.0206),
        ];
        for (year, expected) in cases {
            let jd = jd_for_year(year);
            let actual = delta_t(jd);
            assert!(
                (actual - expected).abs() < 0.02,
                "delta_t at {year}: got {actual}, expected {expected}"
            );
        }
    }

    #[test]
    fn test_delta_t_historical_interpolation() {
        let earlier = delta_t(jd_for_year(1800.0));
        let later = delta_t(jd_for_year(1810.0));
        assert!(earlier.is_finite() && later.is_finite());
    }

    #[test]
    fn test_delta_t_2100_extrapolates_reasonably() {
        let dt = delta_t(jd_for_year(2100.0));
        assert!(dt > 150.0 && dt < 300.0, "delta_t at 2100: {dt}");
    }

    #[test]
    fn test_delta_t_historical_interpolation_is_finite() {
        let earlier = delta_t(jd_for_year(1800.0));
        let later = delta_t(jd_for_year(1810.0));
        assert!(earlier.is_finite() && later.is_finite());
    }
}
