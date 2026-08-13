# xcal — 天文历法核心库

[![Crates.io](https://img.shields.io/crates/v/xcal.svg)](https://crates.io/crates/xcal)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**零外部依赖**的纯 Rust 天文历法库，提供农历、节气、六十甲子、真太阳时计算。

> `xcal` 是 xstars 紫微斗数引擎的天文历法后端，亦可独立用于日历/命理/天文应用。

---

## 功能

- **儒略日** — 公历 ↔ 儒略日双向转换，自动处理 1582 年儒略历/格里高利历分界
- **太阳黄经** — VSOP87 傅里叶级数（376 项），精度 ~0.003°（JPL DE441 验证）
- **月亮黄经** — ELP/MPP02 简化版（627 项），精度 ~0.03°（JPL DE441 验证）
- **节气计算** — 牛顿迭代法搜索 24 节气精确时刻
- **农历转换** — 定朔定气法，UT 日历日对齐，无中气则闰，公历→农历双向转换
- **六十甲子** — 日柱/年柱干支（Tiangan / Dizhi 独立枚举）
- **真太阳时** — 经度校正、时区自动推算（配合地理坐标）
- **历史纪年** — 中国历代年号查询（EraRecord 纪年记录）

---

## 快速开始

```toml
[dependencies]
xcal = "0.1"
```

### 儒略日

```rust
use xcal::JulianDay;

let jd = JulianDay::from_ymd(2000, 1, 1);
println!("{}", jd); // 2451544.5

let (y, m, d) = jd.to_ymd();
assert_eq!((y, m, d), (2000, 1, 1));
```

### 太阳/月亮黄经

```rust
use xcal::{sun_longitude_deg, moon_longitude_deg};

let jd = 2451545.0; // J2000.0
let sun = sun_longitude_deg(jd);  // ≈ 280.46°
let moon = moon_longitude_deg(jd);
```

### 节气

```rust
use xcal::{jieqi, Jieqi, JulianDay};

let jd = jieqi(2024, 0); // 立春 JD
let (y, m, d) = JulianDay(jd).to_ymd();
assert_eq!((y, m, d), (2024, 2, 4));
```

### 农历

```rust
use xcal::solar_to_lunar;

let lunar = solar_to_lunar(2024, 2, 10);
assert_eq!(lunar.month, 1);  // 正月
assert_eq!(lunar.day, 1);    // 初一
```

### 四柱

```rust
use xcal::bazi;

let bazi = bazi(2000, 8, 16, 2, 0, 120.0); // 2000-8-16 02:00 东八区
let (y, m, d, h) = bazi;
println!("年柱={:?} 月柱={:?} 日柱={:?} 时柱={:?}", y, m, d, h);
```

### 六十甲子

```rust
use xcal::{day_ganzhi, Tiangan, Dizhi};

let gz = day_ganzhi(2451544.5); // 2000-01-01
println!("{:?}{:?}", gz.gan, gz.zhi);
```

### 真太阳时

```rust
use xcal::true_solar_time;

let (h, m, s) = true_solar_time(2451544.5, 116.4); // 北京
println!("{}:{}:{}", h, m, s);
```

---

## 架构

```
xcal/
├── Cargo.toml
├── README.md
├── docs/
│   ├── DESIGN.md
│   └── RESEARCH_ASTRONOMY_ENGINE.md
├── tests/
│   ├── accuracy_test.rs
│   └── compare_test.rs
└── src/
    ├── lib.rs
    ├── error.rs
    │
    ├── core/               # 天文算法
    │   ├── eph0.rs         # 太阳/月亮黄经
    │   ├── eph0/           # 傅里叶级数系数表
    │   │   ├── sun_data.rs
    │   │   └── moon_data.rs
    │   ├── delta_t.rs      # ΔT 历史修正
    │   └── constants.rs
    │
    ├── calendar/           # 历法计算
    │   ├── lunar.rs        # 农历核心
    │   ├── jieqi.rs        # 24 节气
    │   ├── ganzhi.rs       # 六十甲子 + 四柱
    │   ├── historical.rs   # 古历修正 + 纪年
    │   ├── era_table.rs    # 年号数据表
    │   └── solar.rs        # 公历工具
    │
    ├── time/               # 时间工具
    │   ├── jd.rs           # 儒略日
    │   └── solar_time.rs   # 真太阳时
    │
    └── geo/                # 地理坐标
        └── location.rs
```

### 数据流

```
公历 ←→ 儒略日 ──→ 天文引擎 ──→ 节气
                │              └──→ 农历（年<1900 整合古历修正）
                │
                ├──→ 真太阳时 ← 地理坐标
                │
                └──→ 历史纪年（年号查询）
```

---

## API 速查

### `xcal`（根级重导出）

常用 API 可从 `xcal::` 直接导入：

| 类型/函数 | 说明 |
|-----------|------|
| `JulianDay(f64)` | 儒略日结构体 |
| `JulianDay::from_ymd(y, m, d)` | 公历 → 儒略日 |
| `JulianDay::to_ymd()` | 儒略日 → 公历 `(y, m, d)` |
| `julian_diff(jd1, jd2)` | 天数差 |
| `solar_to_lunar(y, m, d)` | 公历 → 农历 `LunarDate` |
| `lunar_to_solar(&lunar)` | 农历 → 公历 `(y, m, d)` |
| `LunarDate { year, month, day, is_leap }` | 农历日期结构体 |
| `jieqi(year, n)` | 第 n 个节气的 JD |
| `jieqi_list(year)` | 全年 24 节气 (Jieqi, JD) 列表 |
| `lichun_jd(year)` | 立春 JD |
| `is_before_lichun(jd, year)` | 是否在立春前 |
| `bazi(year, m, d, h, min, lon)` | 四柱八字 |
| `day_ganzhi(jd)` | 日柱干支 |
| `year_ganzhi(year, before_spring)` | 年柱干支 |
| `era_names(year)` | 查询年号/纪年记录 |
| `true_solar_time(jd, longitude)` | 真太阳时 `(h, m, s)` |
| `sun_longitude_deg(jd)` | 太阳黄经（度）|
| `moon_longitude_deg(jd)` | 月亮黄经（度）|

上方“API 速查”列出常用根级 API；算法设计和精度说明见 [`docs/DESIGN.md`](docs/DESIGN.md) 与 [`docs/RESEARCH_ASTRONOMY_ENGINE.md`](docs/RESEARCH_ASTRONOMY_ENGINE.md)。

---

## 精度说明

| 功能 | 当前精度 | 说明 |
|------|---------|------|
| 太阳黄经 | ~0.003°（JPL DE441） | VSOP87 376 项余弦级数，精度良好 |
| 月亮黄经 | ~0.03°（JPL DE441） | ELP/MPP02 627 项，近现代 <0.03° |
| 节气计算 | ~2 小时（太阳黄经 0.003° 误差） | 太阳精度高，但与 DE441 仍有微小偏差 |
| 农历朔日 | ~0.5 天（新月边界偏移） | 少数日期新月球与公历日界相差 ~0.5 天，导致 day_in 差 1 |

**精度瓶颈**：月亮 627 项模型在少数月份的新月时刻仍有 ~0.5 天偏差。这是 UT 日历日分界处的问题（新月发生在 UT 午夜前后半天的任何时刻时，`to_ymd()` 可能取不同的日历日）。后续可参考 JPL DE440/DE441 精确历表做后处理修正。

---

## 验证

```bash
cargo test -p xcal
cargo clippy --all-targets -p xcal -- -D warnings
```

当前通过 33 个 xcal 单元/集成测试 + 215 个 workspace 全量测试。

---

## 待办（TODO）

| # | 任务 | 优先级 | 状态 |
|---|------|--------|------|
| 1 | 性能基准测试 | 低 | ✅ 已完成（xstars/benches/astrolabe.rs） |
| 2 | 提升新月时刻精度 | 中 | 🚧 月亮 0.5 天误差是下一目标 |

---

## 技术来源

- **Jean Meeus**《天文算法》(Astronomical Algorithms)
- **ELP2000-82** 月球理论（主要周期项简化版 + MPP02 修正）
- **IAU 1980** 章动理论
- **NASA/IAU** ΔT 多项式拟合公式
- **许剑伟（寿星万年历 sxwnl）** 部分数据和算法来源

## 许可证

MIT
