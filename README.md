# xcal — 天文历法核心库

[![Crates.io](https://img.shields.io/crates/v/xcal.svg)](https://crates.io/crates/xcal)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

纯 Rust 天文历法库，默认 feature 无外部依赖，提供农历、节气、六十甲子、真太阳时和历史纪年计算。启用 `serde` feature 时引入可选 `serde` 依赖。

---

## 功能

- **儒略日** — 公历 ↔ 儒略日双向转换，处理儒略历/格里高利历分界
- **太阳黄经** — VSOP87 傅里叶级数，376 项
- **月亮黄经** — ELP/MPP02 简化模型，627 项
- **节气计算** — 太阳黄经求根，计算 24 节气时刻
- **农历转换** — 定朔定气、无中气置闰、公历 ↔ 农历双向转换，按 UTC+8 日界对齐
- **六十甲子** — 日柱、年柱、月柱、时柱及 `Bazi`
- **真太阳时** — 经度校正、均时差、时区推算
- **历史历法** — 历史朔望月修正和中国历代年号查询

## 安装

```toml
[dependencies]
xcal = "0.1"
```

启用 `serde`：

```toml
[dependencies]
xcal = { version = "0.1", features = ["serde"] }
```

---

## 快速开始

### 儒略日

```rust
use xcal::JulianDay;

let jd = JulianDay::from_ymd(2000, 1, 1);
assert_eq!(jd.to_ymd(), (2000, 1, 1));
assert_eq!(jd.0, 2451544.5);
```

### 太阳/月亮黄经

```rust
use xcal::core::eph0::{moon_longitude_deg, sun_longitude_deg};

let jd = 2451545.0; // J2000.0
let sun = sun_longitude_deg(jd);
let moon = moon_longitude_deg(jd);
println!("太阳={}°，月亮={}°", sun, moon);
```

### 节气

```rust
use xcal::{jieqi, JulianDay};

let jd = jieqi(2024, 0); // 立春 JD
let date = JulianDay(jd).to_ymd();
assert_eq!(date, (2024, 2, 4));
```

### 农历

```rust
use xcal::solar_to_lunar;

let lunar = solar_to_lunar(2024, 2, 10);
assert_eq!((lunar.year, lunar.month, lunar.day, lunar.is_leap), (2024, 1, 1, false));
```

### 四柱

```rust
use xcal::bazi;

let result = bazi(2000, 8, 16, 2, 0, 120.0); // 标准时，东八区
println!(
    "年柱={:?} 月柱={:?} 日柱={:?} 时柱={:?}",
    result.year, result.month, result.day, result.hour
);
```

### 六十甲子

```rust
use xcal::day_ganzhi;

let ganzhi = day_ganzhi(2451544.5); // 2000-01-01
println!("{:?}{:?}", ganzhi.gan, ganzhi.zhi);
```

### 真太阳时

```rust
use xcal::apparent_solar_time;

let (hour, minute, second) = apparent_solar_time(2451544.5, 116.4); // 北京
println!("{hour}:{minute}:{second}");
```

---

## 架构

```text
xcal/
├── Cargo.toml
├── README.md
├── tests/
│   ├── accuracy_test.rs
│   └── compare_test.rs
└── src/
    ├── lib.rs
    ├── error.rs
    ├── core/               # 天文算法
    │   ├── eph0.rs         # 太阳/月亮黄经
    │   ├── eph0/           # 傅里叶级数系数表
    │   │   ├── sun_data.rs
    │   │   └── moon_data.rs
    │   ├── delta_t.rs      # ΔT 历史修正公式
    │   └── constants.rs
    ├── calendar/           # 历法计算
    │   ├── lunar.rs        # 农历核心
    │   ├── jieqi.rs        # 24 节气
    │   ├── ganzhi.rs       # 六十甲子与四柱
    │   ├── historical.rs   # 历史历法与年号查询
    │   ├── era_table.rs    # 年号数据表
    │   └── solar.rs        # 公历工具
    ├── time/               # 时间工具
    │   ├── jd.rs           # 儒略日
    │   └── solar_time.rs   # 真太阳时
    └── geo/                # 地理坐标
        └── location.rs
```

数据流：

```text
公历 ←→ 儒略日 ──→ 天文引擎 ──→ 节气
                │              └──→ 农历
                │
                ├──→ 真太阳时 ← 地理坐标
                │
                └──→ 历史历法与年号
```

模块按数据流拆分：`core` 提供天文算法，`calendar` 提供历法逻辑，`time` 提供时间工具，`geo` 提供经纬度和时区辅助。`lib.rs` 重导出主要公共 API，crate 可独立发布到 crates.io。

---

## API 速查

### 日期与时间

| API | 说明 |
|---|---|
| `JulianDay(f64)` | 儒略日结构体 |
| `JulianDay::from_ymd(y, m, d)` | 公历 → 儒略日 |
| `JulianDay::to_ymd()` | 儒略日 → 公历 `(y, m, d)` |
| `julian_diff(jd1, jd2)` | 两个 `JulianDay` 的天数差 |
| `apparent_solar_time(jd, longitude)` | 真太阳时 `(h, m, s)` |
| `convert_to_apparent_solar_time(...)` | 标准时 → 真太阳时日期时间 |
| `Location::new(longitude, latitude)` | 创建地理位置 |

### 天文算法

| API | 说明 |
|---|---|
| `xcal::core::eph0::sun_longitude_deg(jd)` | 太阳真黄经，单位为度 |
| `xcal::core::eph0::moon_longitude_deg(jd)` | 月亮黄经，单位为度 |
| `xcal::core::eph0::nutation_longitude(jd)` | IAU 2000B 黄经章动，单位为弧度 |
| `delta_t(jd)` | ΔT 估算值，输入儒略日 |

### 节气、农历与历史历法

| API | 说明 |
|---|---|
| `jieqi(year, n)` | 第 `n` 个节气的 JD |
| `jieqi_list(year)` | 全年 24 节气 `(Jieqi, JD)` 列表 |
| `lichun_jd(year)` | 立春 JD |
| `is_before_lichun(jd, year)` | 判断是否在立春之前 |
| `solar_to_lunar(y, m, d)` | 公历 → `LunarDate` |
| `lunar_to_solar(&lunar)` | `LunarDate` → 公历 `(y, m, d)` |
| `xcal::calendar::historical::era_names(year)` | 查询年号/纪年记录 |

### 干支

| API | 说明 |
|---|---|
| `bazi(year, m, d, h, min, longitude)` | 返回 `Bazi` 四柱结构体 |
| `day_ganzhi(jd)` | 日柱干支 |
| `year_ganzhi(year, before_spring)` | 年柱干支 |
| `Tiangan` / `Dizhi` | 天干、地支枚举 |
| `GanZhi` / `Bazi` | 干支组合、四柱结构体 |

---

## 算法与设计决策

### 儒略日与历制分界

`1582-10-15` 起使用格里高利历公式，此前使用儒略历公式。儒略日是各模块之间的统一时间表示。

### 太阳模型：VSOP87

太阳黄经使用 Bretagnon 和 Francou 发布的 VSOP87 地球轨道级数，共 6 层 376 项：

| 层 | 项数 | 权重 |
|---|---:|---:|
| L0 | 186 | `t⁰` |
| L1 | 114 | `t¹` |
| L2 | 60 | `t²` |
| L3 | 8 | `t³` |
| L4 | 4 | `t⁴` |
| L5 | 4 | `t⁵` |

实现依次加入地心修正、岁差微调、太阳光行差和 IAU 2000B 章动，返回太阳真黄经。

### 月亮模型：ELP/MPP02

月亮黄经使用 ELP/MPP02 简化级数，共 4 层 627 项：

| 层 | 项数 | 权重 |
|---|---:|---:|
| ML0 | 442 | `t⁰` |
| ML1 | 149 | `t¹` |
| ML2 | 34 | `t²` |
| ML3 | 2 | `t³` |

实现加入月亮光行差和 IAU 2000B 章动。当前模型用于节气、朔日和农历日期计算，不承诺高精度天文历表替代精度。

### 节气

传统节气序号以立春为第 0 个节气，黄经映射为：

```text
黄经 = ((序号 + 21) % 24) × 15°
立春 315°，春分 0°，夏至 90°，秋分 180°，冬至 270°
```

先用平均太阳运动取得近似 JD，再用太阳黄经误差进行最多 12 次迭代求根。

### 农历

采用定朔定气法，不使用固定月长或静态近现代表：

1. 以冬至所在月建立锚定年，并使用已知朔日纪元生成连续朔日。
2. 由太阳黄经判断每个朔望月是否包含中气。
3. 无中气的月份判为闰月。
4. 新月日期按中国标准时间 `UTC+8` 午夜对齐，再计算农历日序。
5. 公元 1900 年以前优先尝试历史修正表，覆盖约 `-722..1899` 的历史范围。

朔日使用平均朔望月作为初值，再通过日月黄经差的数值迭代精化。农历 API 的目标是日期和月序，不是输出完整天文历表。

### 设计取舍

- 默认 feature 无外部依赖，便于作为独立 crate 发布。
- `serde` 仅作为可选 feature，不让序列化依赖进入默认构建。
- 不引入 DE440/DE441 数值历表：当前接口只需要节气、朔日和日期级农历结果，额外历表会显著增加数据体积和实现复杂度。
- ΔT 公式保留为独立模块，供历史时间换算场景使用；当前历法主流程以儒略日和模型计算为核心。

---

## 精度、验证与限制

当前文档只声明代码和测试能够直接支撑的范围：

| 项目 | 当前实现/验证范围 |
|---|---|
| 太阳黄经 | VSOP87，376 项；集成测试提供 JPL DE441 参考点，容差 `<0.003°` |
| 月亮黄经 | ELP/MPP02，627 项；集成测试提供 JPL DE441 参考点，容差 `<0.03°` |
| 节气 | 测试覆盖 2024 年节气日期、顺序和立春边界；未承诺分钟级误差 |
| 农历 | 测试覆盖 UTC+8 日界、已知节日、闰月和公历/农历往返；未承诺全历史范围的权威历表等价性 |
| 历史历法 | 使用历史修正表和年号表；适用范围以数据表覆盖范围为准 |

已知限制：

- 月亮级数是面向日历计算的简化模型，不是 JPL 数值积分历表。
- ΔT 对古代日期存在观测和模型不确定性，历史时间结果应视为估算。
- 现有测试覆盖代表性日期，不等同于对所有年份、时区和历法边界的穷举验证。

后续方向：扩大月亮级数、增加权威历表差分测试、补充性能基准和更长时间跨度验证。当前实现优先保持纯 Rust 和小型数据体积。

验证命令：

```bash
cargo fmt --check
cargo test -p xcal
cargo test --doc -p xcal
cargo clippy --all-targets -p xcal -- -D warnings
cargo doc --no-deps -p xcal
cargo package --list -p xcal
```

---

## 技术来源

- Bretagnon P.; Francou G. (1987), **VSOP87**
- Chapront J. 等 (2002), **ELP/MPP02**
- Jean Meeus, *Astronomical Algorithms*
- IAU 2000B 章动模型
- NASA/IAU ΔT 估算公式
- 许剑伟，寿星万年历（sxwnl）相关数据和算法参考

## 许可证

MIT
