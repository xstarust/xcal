//! 古历修正系统
//!
//! ## 算法概述
//!
//! 4 区间调度算法，用于 -722 ~ 1960 年
//! 的农历日期还原。
//!
//! ### 4 区间调度
//!
//! | 区间 | 范围 | 算法 | 精度 |
//! |------|------|------|------|
//! | 1 | jd < -722 | 牛顿迭代（VSOP87/ELP） | 高 |
//! | 2 | -722 ≤ jd < 619 | KB 平气平朔线性拟合 | 整日级（±0 天） |
//! | 3 | 619 ≤ jd < 1960 | rough_shuo/qi + 修正表 ±1 天 | 整日级（±0 天） |
//! | 4 | jd ≥ 1960 | 牛顿迭代（VSOP87/ELP） | 高 |
//!
//! ### KB 线性拟合（区间 2）
//!
//! 不同朝代使用不同的历法公式，每个时期用一段直线 `D = B + K × n`
//! 拟合平气/平朔。`K` = 斜率（朔望月长或节气间隔），`B` = 纪元 JD。
//!
//! 整套参数表覆盖了从古历·春秋到元·授时历共 46 段历法。
//!
//! ### 低精度 + 修正表（区间 3）
//!
//! 用 5 项三角公式快速估算朔日/节气时刻（~±12h 精度），再查修正表
//! 补齐历史偏差。修正表将古代官历的
//! 朔日偏差编码为每位 '0'/'1'/'2'（不变/+1 天/-1 天）。
//!
//! - 朔修正表 SB：16598 位（619~1960 年）
//! - 气修正表 QB：7567 位（1645~1960 年）
//!
//! 修正表用 run-length 编码压缩：字母代表 0 的重复数 + 可选后缀。

use crate::calendar::era_table::find_era;
use crate::calendar::ganzhi::bazi;
use crate::calendar::lunar::{LunarDate, lunar_to_solar};
use crate::core::constants::{J2000, PI};
use crate::core::eph0::{moon_sun_delta_deg, sun_longitude_deg};
use crate::time::jd::JulianDay;
use std::sync::OnceLock;

/// 现代天文算法分界：1960-01-15（此日期后改用牛顿迭代高精度）
const MODERN_EPOCH_JD: f64 = 2436935.0;

/// KB 辅助函数：(纪元 JD + J2000, 斜率)
const fn kb(epoch: f64, slope: f64) -> (f64, f64) {
    (epoch + J2000, slope)
}

// ─── 平朔（new moon）KB 直线拟合参数 ───
// 每段：(纪元 JD + J2000, 朔望月长度)

const SHUO_KB_TABLE: &[(f64, f64)] = &[
    kb(1457698.231017, 29.53067166),
    kb(1546082.512234, 29.53085106),
    kb(1640640.735300, 29.53060000),
    kb(1642472.151543, 29.53085439),
    kb(1683430.509300, 29.53086148),
    kb(1752148.041079, 29.53085097),
    kb(1807724.481520, 29.53059851),
    kb(1883618.114100, 29.53060000),
    kb(1907360.704700, 29.53060000),
    kb(1936596.224900, 29.53060000),
    kb(1939135.675300, 29.53060000),
];
/// 平朔 KB 终止边界：619-01-21（之后用低精度+修正表）
const SHUO_KB_END: f64 = J2000 + 1947168.0;

/// 低精度+修正表起点偏置（朔）：EPOCH_SHUO_JD - 14天
const SHUO_KB_OFFSET: f64 = 14.0;

/// 平气（solar term）KB 直线拟合参数
const QI_KB_TABLE: &[(f64, f64)] = &[
    kb(1640650.479938, 15.21842500),
    kb(1642476.703182, 15.21874996),
    kb(1683430.515601, 15.218750011),
    kb(1752157.640664, 15.218749978),
    kb(1807675.003759, 15.218620279),
    kb(1883627.765182, 15.218612292),
    kb(1907369.128100, 15.218449176),
    kb(1936603.140413, 15.218425000),
    kb(1939145.524180, 15.218466998),
    kb(1947180.798300, 15.218524844),
    kb(1964362.041824, 15.218533526),
    kb(1987372.340971, 15.218513908),
    kb(1999653.819126, 15.218530782),
    kb(2007445.469786, 15.218535181),
    kb(2021324.917146, 15.218526248),
    kb(2047257.232342, 15.218519654),
    kb(2070282.898213, 15.218425000),
    kb(2073204.872850, 15.218515221),
    kb(2080144.500926, 15.218530782),
    kb(2086703.688963, 15.218523776),
    kb(2110033.182763, 15.218425000),
    kb(2111190.300888, 15.218425000),
    kb(2113731.271005, 15.218515671),
    kb(2120670.840263, 15.218425000),
    kb(2123973.309063, 15.218425000),
    kb(2125068.997336, 15.218477932),
    kb(2136026.312633, 15.218472436),
    kb(2156099.495538, 15.218425000),
    kb(2159021.324663, 15.218425000),
    kb(2162308.575254, 15.218461742),
    kb(2178485.706538, 15.218425000),
    kb(2178759.662849, 15.218445786),
    kb(2185334.020800, 15.218425000),
    kb(2187525.481425, 15.218425000),
    kb(2188621.191481, 15.218437494),
];
/// 平气 KB 终止边界：1645-09-21（之后用低精度+修正表）
const QI_KB_END: f64 = J2000 + 2322147.76;
/// 低精度+修正表起点偏置（气）：-7天
const QI_KB_OFFSET: f64 = 7.0;

const SB_ENC: &str = concat!(
    "EqoFscDcrFpmEsF2DfFideFelFpFfFfFiaipqti1ksttikptikqckstekqttgkqttgkqteksttikptikq2fjstgjqttjkqttgkqt",
    "ekstfkptikq2tijstgjiFkirFsAeACoFsiDaDiADc1AFbBfgdfikijFifegF1FhaikgFag1E2btaieeibggiffdeigFfqDfaiBkF",
    "1kEaikhkigeidhhdiegcFfakF1ggkidbiaedksaFffckekidhhdhdikcikiakicjF1deedFhFccgicdekgiFbiaikcfi1kbFibef",
    "gEgFdcFkFeFkdcfkF1kfkcickEiFkDacFiEfbiaejcFfffkhkdgkaiei1ehigikhdFikfckF1dhhdikcfgjikhfjicjicgiehdik",
    "cikggcifgiejF1jkieFhegikggcikFegiegkfjebhigikggcikdgkaFkijcfkcikfkcifikiggkaeeigefkcdfcfkhkdgkegieid",
    "hijcFfakhfgeidieidiegikhfkfckfcjbdehdikggikgkfkicjicjF1dbidikFiggcifgiejkiegkigcdiegfggcikdbgfgefjF1",
    "kfegikggcikdgFkeeijcfkcikfkekcikdgkabhkFikaffcfkhkdgkegbiaekfkiakicjhfgqdq2fkiakgkfkhfkfcjiekgFebicg",
    "gbedF1jikejbbbiakgbgkacgiejkijjgigfiakggfggcibFifjefjF1kfekdgjcibFeFkijcfkfhkfkeaieigekgbhkfikidfcje",
    "aibgekgdkiffiffkiakF1jhbakgdki1dj1ikfkicjicjieeFkgdkicggkighdF1jfgkgfgbdkicggfggkidFkiekgijkeigfiski",
    "ggfaidheigF1jekijcikickiggkidhhdbgcfkFikikhkigeidieFikggikhkffaffijhidhhakgdkhkijF1kiakF1kfheakgdkif",
    "iggkigicjiejkieedikgdfcggkigieeiejfgkgkigbgikicggkiaideeijkefjeijikhkiggkiaidheigcikaikffikijgkiahi1",
    "hhdikgjfifaakekighie1hiaikggikhkffakicjhiahaikggikhkijF1kfejfeFhidikggiffiggkigicjiekgieeigikggiffig",
    "gkidheigkgfjkeigiegikifiggkidhedeijcfkFikikhkiggkidhh1ehigcikaffkhkiggkidhh1hhigikekfiFkFikcidhh1hit",
    "cikggikhkfkicjicghiediaikggikhkijbjfejfeFhaikggifikiggkigiejkikgkgieeigikggiffiggkigieeigekijcijikgg",
    "ifikiggkideedeijkefkfckikhkiggkidhh1ehijcikaffkhkiggkidhh1hhigikhkikFikfckcidhh1hiaikgjikhfjicjicgie",
    "hdikcikggifikigiejfejkieFhegikggifikiggfghigkfjeijkhigikggifikiggkigieeijcijcikfksikifikiggkidehdeij",
    "cfdckikhkiggkhghh1ehijikifffffkhsFngErD1pAfBoDd1BlEtFqA2AqoEpDqElAEsEeB2BmADlDkqBtC1FnEpDqnEmFsFsAFn",
    "llBbFmDsDiCtDmAB2BmtCgpEplCpAEiBiEoFqFtEqsDcCnFtADnFlEgdkEgmEtEsCtDmADqFtAFrAtEcCqAE1BoFqC1F1DrFtBmF",
    "tAC2ACnFaoCgADcADcCcFfoFtDlAFgmFqBq2bpEoAEmkqnEeCtAE1bAEqgDfFfCrgEcBrACfAAABqAAB1AAClEnFeCtCgAADqDoB",
    "mtAAACbFiAAADsEtBqAB2FsDqpFqEmFsCeDtFlCeDtoEpClEqAAFrAFoCgFmFsFqEnAEcCqFeCtFtEnAEeFtAAEkFnErAABbFkAD",
    "nAAeCtFeAfBoAEpFtAABtFqAApDcCGJ",
);
const QB_ENC: &str = concat!(
    "FrcFs22AFsckF2tsDtFqEtF1posFdFgiFseFtmelpsEfhkF2anmelpFlF1ikrotcnEqEq2FfqmcDsrFor22FgFrcgDscFs22FgEe",
    "FtE2sfFs22sCoEsaF2tsD1FpeE2eFsssEciFsFnmelpFcFhkF2tcnEqEpFgkrotcnEqrEtFermcDsrE222FgBmcmr22DaEfnaF22",
    "2sD1FpeForeF2tssEfiFpEoeFssD1iFstEqFppDgFstcnEqEpFg11FscnEqrAoAF2ClAEsDmDtCtBaDlAFbAEpAAAAAD2FgBiBqo",
    "BbnBaBoAAAAAAAEgDqAdBqAFrBaBoACdAAf1AACgAAAeBbCamDgEifAE2AABa1C1BgFdiAAACoCeE1ADiEifDaAEqAAFe1AcFbcA",
    "AAAAF1iFaAAACpACmFmAAAAAAAACrDaAAADG0",
);

/// 解码修正表压缩串（run-length 编码）
///
/// 编码规则利用 ASCII 连续性，每个字母映射到特定输出：
///
/// | 区间 | 计算方式 | 输出 | 示例 |
/// |------|---------|------|------|
/// | A~F | `(F - c + 1) × 10` 个零 | 60/50/40/30/20/10 个 0 | A→60个0, F→10个0 |
/// | G~J | `(J - c + 2)` 个零 | 5/4/3/2 个 0 | G→5个0, J→2个0 |
/// | k, i, j, h | 固定短串 | `01`/`001`/`0101`/`001001` | — |
/// | a~g | `(g - c + 3)` 个零 + `1` | 9~3 个零+1 | a→9个0+1, g→3个0+1 |
/// | l~t | `(t - c + 1)` 个零 + `2` | 9~1 个零+2 | l→9个0+2, t→1个0+2 |
/// | 0~2 | 数字字面量 | 原样输出 | — |
fn decompress(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 3);
    let zeros = |n: usize| std::iter::repeat_n(b'0', n);

    for c in s.chars() {
        // 大写 A~J: 连续零串，A=60 个 0（最长），J=2 个 0（最短）
        if ('A'..='F').contains(&c) {
            out.extend(zeros(((b'F' - c as u8 + 1) * 10) as usize));
        } else if ('G'..='J').contains(&c) {
            out.extend(zeros((b'J' - c as u8 + 2) as usize));
        // 小写 a~g: n 个 0 + '1'，a=9零+1，g=3零+1
        } else if ('a'..='g').contains(&c) {
            out.extend(zeros((b'g' - c as u8 + 3) as usize));
            out.push(b'1');
        } else if ('l'..='t').contains(&c) {
            // n个0+'2'，l=9零+2，t=1零+2
            out.extend(zeros((b't' - c as u8 + 1) as usize));
            out.push(b'2');
        } else {
            // 固定短串（不满足连续规律）
            match c {
                'k' => out.extend_from_slice(b"01"),
                'i' => out.extend_from_slice(b"001"),
                'j' => out.extend_from_slice(b"0101"),
                'h' => out.extend_from_slice(b"001001"),
                '0'..='2' => out.push(c as u8),
                _ => {}
            }
        }
    }
    out
}

fn sb_table() -> &'static [u8] {
    static SB: OnceLock<Vec<u8>> = OnceLock::new();
    SB.get_or_init(|| decompress(SB_ENC))
}

fn qb_table() -> &'static [u8] {
    static QB: OnceLock<Vec<u8>> = OnceLock::new();
    QB.get_or_init(|| decompress(QB_ENC))
}

/// 快速估算朔日时刻（5 项三角公式）
///
/// 输入 `w` = 月日黄经差（弧度），第 n 个朔传入 n × 2π。
/// 返回 J2000 偏移 JD + 北京时 8h。
///
/// 公式由来（so_low）：
/// 1. 平朔近似：t = (W + 1.08472) / 7771.377
/// 2. 三项摄动修正（月亮受太阳引力摄动的主要周期项）
/// 3. ΔT 近似修正
///
/// 精度约 ±12h（历史上 619~1960 年期间可接受，修正表会补到整日级）。
fn rough_shuo(w: f64) -> f64 {
    let v = 7771.37714500204;
    let mut t = (w + 1.08472) / v;
    t -= (-0.0000331 * t * t
        + 0.10976 * (0.785 + 8328.6914 * t).cos()
        + 0.02224 * (0.187 + 7214.0629 * t).cos()
        - 0.03342 * (4.669 + 628.3076 * t).cos())
        / v;
    t += (32.0 * (t + 1.8) * (t + 1.8) - 20.0) / 86400.0 / 36525.0;
    t * 36525.0 + 8.0 / 24.0
}

/// 快速估算节气时刻（5 项三角公式）
///
/// 输入 `w` = 太阳黄经（弧度），第 n 个节气传入 n × π/12。
/// 返回 J2000 偏移 JD + 北京时 8h。
///
/// 公式由来（qi_low）：
/// 1. 平气近似：t = (W - 4.895) / 628.33
/// 2. 椭圆轨道修正 + 泊松项
/// 3. 光行差与章动修正
///
/// 最大误差 < 30 分钟，平均 5 分。
fn rough_qi(w: f64) -> f64 {
    let v = 628.3319653318;
    let mut t = (w - 4.895062166) / v;
    t -= (53.0 * t * t
        + 334116.0 * (4.67 + 628.307585 * t).cos()
        + 2061.0 * (2.678 + 628.3076 * t).cos() * t)
        / v
        / 1e7;
    let l = 48950621.66
        + 6283319653.318 * t
        + 53.0 * t * t
        + 334166.0 * (4.669257 + 628.307585 * t).cos()
        + 3489.0 * (4.6261 + 1256.61517 * t).cos()
        + 2060.6 * (2.67823 + 628.307585 * t).cos() * t
        - 994.0
        - 834.0 * (2.1824 - 33.75705 * t).sin();
    t -= (l / 1e7 - w) / 628.332 + (32.0 * (t + 1.8) * (t + 1.8) - 20.0) / 86400.0 / 36525.0;
    t * 36525.0 + 8.0 / 24.0
}

/// KB 直线拟合：用 `D = B + K × floor((jd + offset - B) / K)` 计算气/朔日
///
/// 遍历 KB 表，找到 jd + offset 落入的历法段，外推取整。
/// `special` 参数用于太初历边界特殊修正（-103-01-24 vs -103-01-23）。
fn calc_kb(table: &[(f64, f64)], end: f64, jd_abs: f64, offset: f64, special: Option<f64>) -> f64 {
    for i in 0..table.len() {
        let (epoch, slope) = table[i];
        let next_epoch = if i + 1 < table.len() {
            table[i + 1].0
        } else {
            end
        };
        if jd_abs + offset < next_epoch {
            let n = ((jd_abs + offset - epoch) / slope).floor();
            let mut d = epoch + slope * n;
            d = (d + 0.5).floor();
            if special.is_some_and(|sp| (d - sp).abs() < 1.0) {
                d += 1.0;
            }
            return d;
        }
    }
    let (epoch, slope) = table[table.len() - 1];
    let n = ((jd_abs + offset - epoch) / slope).floor();
    let mut d = epoch + slope * n;
    d = (d + 0.5).floor();
    d
}

/// 低精度估算 + 修正表：用三角公式算近似值，再查修正表 ±1 天补齐历史偏差
///
/// 索引 `(jd - start) / step` 定位修正表的位号。
fn calc_correct(
    angle: f64,               // 粗略角度（弧度），第 n 个朔传入 n×2π，第 n 个气传入 n×π/12
    jd: f64,                  // 目标 JD，用于定位修正表所属的年份
    start: f64,               // 修正表起始 JD
    step: f64,                // 修正表每位的跨度（朔=29.53天，气≈15.22天）
    table: &[u8],             // 修正表数据（'0'=不变, '1'=+1天, '2'=-1天）
    rough_fn: fn(f64) -> f64, // 低精度三角估算函数（rough_shuo 或 rough_qi）
) -> f64 {
    let guess = rough_fn(angle);
    let mut result = (guess + 0.5).floor() + J2000;
    let idx = ((jd - start) / step).floor() as usize;
    if let Some(&b) = table.get(idx) {
        match b {
            b'1' => result += 1.0,
            b'2' => result -= 1.0,
            _ => {}
        }
    }
    result
}

/// 牛顿迭代找朔日（VSOP87 高精度）
///
/// 用 `find_new_moon` 相同的算法：从平朔近似出发，牛顿迭代解
/// `moon_sun_delta_deg(t) = 0`。数值微分求导。
/// 用于区间 1 和 4（纯天文计算）。
fn newton_shuo(jd_approx: f64) -> f64 {
    let n = ((jd_approx - 2451550.257) / 29.530588853).round() as i32;
    let mut t = 2451550.257 + n as f64 * 29.530588853;
    for _ in 0..10 {
        let d = moon_sun_delta_deg(t);
        if d.abs() < 1e-6 {
            break;
        }
        let ep = 0.01;
        let der = (moon_sun_delta_deg(t + ep) - moon_sun_delta_deg(t - ep)) / (2.0 * ep);
        if der.abs() < 1e-10 {
            break;
        }
        t -= d / der;
    }
    t
}

/// 牛顿迭代找节气时刻（VSOP87 高精度）
///
/// 从近似 JD 出发，用太阳黄经偏差 `diff = sun_lon - target_lon`
/// 迭代修正：`jd -= diff × 365.25 / 360`。
/// 用于区间 1 和 4（纯天文计算）。
fn newton_qi(jd_approx: f64, target_lon: f64) -> f64 {
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

/// 计算朔日（新月）时刻（4 区间调度自动选择算法）
///
/// 朔 = 日月黄经相合（月亮追上太阳），定农历月的初一。
/// 根据 JD 所在年代自动调度算法。
pub fn calc_shuo(jd_abs: f64) -> f64 {
    let kb_bound = SHUO_KB_TABLE[0].0 - SHUO_KB_OFFSET;
    let correct_bound = SHUO_KB_END - SHUO_KB_OFFSET;
    if jd_abs < kb_bound || jd_abs >= MODERN_EPOCH_JD {
        return newton_shuo(jd_abs);
    }
    if jd_abs < correct_bound {
        return calc_kb(
            SHUO_KB_TABLE,
            SHUO_KB_END,
            jd_abs,
            SHUO_KB_OFFSET,
            Some(J2000 + 1683460.0),
        );
    }
    let sb = sb_table();
    let shuo_angle = ((jd_abs + SHUO_KB_OFFSET - J2000 - 2451551.0) / 29.5306).floor() * 2.0 * PI;
    calc_correct(shuo_angle, jd_abs, correct_bound, 29.5306, sb, rough_shuo)
}

/// 计算气（太阳过中气）的时刻（4 区间调度自动选择算法）
///
/// 气 = 太阳黄经到达 30° 整数倍的时刻，用于定月建和闰月。
/// 与 calc_shuo 相同的算法调度。target_lon 为目标黄经（度），
/// 如冬至=270°、春分=0°、夏至=90°、秋分=180°。
pub fn calc_qi(jd_abs: f64, target_lon: f64) -> f64 {
    let kb_bound = QI_KB_TABLE[0].0 - QI_KB_OFFSET;
    let correct_bound = QI_KB_END - QI_KB_OFFSET;
    if jd_abs < kb_bound || jd_abs >= MODERN_EPOCH_JD {
        return newton_qi(jd_abs, target_lon);
    }
    if jd_abs < correct_bound {
        return calc_kb(QI_KB_TABLE, QI_KB_END, jd_abs, QI_KB_OFFSET, None);
    }
    let qb = qb_table();
    let qi_angle =
        ((jd_abs + QI_KB_OFFSET - J2000 - 2451259.0) / 365.2422 * 24.0).floor() * PI / 12.0;
    calc_correct(
        qi_angle,
        jd_abs,
        correct_bound,
        365.2422 / 24.0,
        qb,
        rough_qi,
    )
}

/// 古历法的历史时期枚举
///
/// 不同时期使用不同的历法规则和正月起点：
/// - `Standard`（-103 年太初历至今）：正月=寅月（冬至后第2个月），秦汉以后
/// - `QinHan`（-221 ~ -104）：秦至汉初，正月=亥月（冬至前1个月），岁首十月
/// - `WarringStates`（-479 ~ -221）：战国时期，正月=子月（冬至月）
/// - `SpringAutumn`（-721 ~ -479）：春秋时期，正月=子月（冬至月）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalEra {
    /// 汉武帝太初历以来（-103 年至今），正月固定在寅月
    Standard,
    /// 春秋时期（-721 ~ -479），正月在子月（冬至月）
    SpringAutumn,
    /// 战国时期（-479 ~ -221），正月在子月（冬至月）
    WarringStates,
    /// 秦至汉初（-221 ~ -104），正月在亥月，岁首为十月
    QinHan,
}

impl HistoricalEra {
    pub fn from_year(year: i32) -> Self {
        if year >= -104 {
            Self::Standard
        } else if year >= -221 {
            Self::QinHan
        } else if year >= -479 {
            Self::WarringStates
        } else if year >= -721 {
            Self::SpringAutumn
        } else {
            Self::Standard
        }
    }
}

/// 古历系统的月索引 → 农历月号
///
/// 月索引从冬至月（十一月）开始：0=十一月, 1=十二月, 2=正月...
/// 不同历史时期的一月位置不同，通过 HistoricalEra 调整偏移。
/// 返回 (农历月号 1~12, 闰月前缀名称如"闰"/"后九"/"十三")。
pub fn lunar_month_from_history(
    month_index: usize,
    leap: Option<usize>,
    is_leap: bool,
    era: HistoricalEra,
) -> (u32, &'static str) {
    let shift = match era {
        HistoricalEra::Standard => 10,
        _ => 11,
    };
    let month_raw = if is_leap {
        if leap.is_some() {
            ((month_index + shift - 1 + 12) % 12 + 1) as u32
        } else {
            ((month_index + shift + 12) % 12 + 1) as u32
        }
    } else if let Some(lo) = leap {
        if month_index > lo {
            ((month_index + shift - 1 + 12) % 12 + 1) as u32
        } else {
            ((month_index + shift + 12) % 12 + 1) as u32
        }
    } else {
        ((month_index + shift + 12) % 12 + 1) as u32
    };
    let leap_name = if is_leap {
        match era {
            HistoricalEra::SpringAutumn => "\u{5341}\u{4e09}",
            HistoricalEra::WarringStates | HistoricalEra::QinHan => "\u{540e}\u{4e5d}",
            HistoricalEra::Standard => "\u{95f0}",
        }
    } else {
        ""
    };
    (month_raw, leap_name)
}

type LunarContext = (Vec<(i32, u32, u32)>, Option<usize>, HistoricalEra);

/// 计算农历年周期的连续 15 个朔日
///
/// year = 冬至所在的公历年份。该冬至对应农历同年十一月，
/// 因此这个值同时也是该农历年的编号。
/// 例如 year=2024 → 从 2024-12-21 冬至月开始的农历年周期。
///
/// 返回 15 个连续朔日（覆盖从冬至月十一月到次年十二月的约15个月）、
/// 闰月索引（如闰二月则为 3）、历史时期枚举。
/// 仅支持 -722..1900 年，超出返回 None。
pub fn shuo_list(year: i32) -> Option<LunarContext> {
    if !(-722..1900).contains(&year) {
        return None;
    }
    let era = HistoricalEra::from_year(year);
    // 冬至约在12月21-23日，取12月1日作为牛顿迭代的起点
    let guess = JulianDay::from_ymd(year, 12, 1).0;
    // 精确计算冬至时刻：太阳黄经到达270°的时刻
    let dongzhi_jd = calc_qi(guess, 270.0);
    // items: 连续 15 个农历月的初一（朔日）公历日期。
    // 15 个月的跨度确保覆盖整个农历年（12 个月 + 可能闰月 = 最多 13 个月）
    let mut items = Vec::with_capacity(15);
    let start = dongzhi_jd - 30.0;
    let start_index = ((start - (J2000 + 2451550.257)) / 29.530588853).ceil() as i32;
    for i in 0..15 {
        let approx = J2000 + 2451550.257 + (start_index + i) as f64 * 29.530588853;
        let shuo_jd = calc_shuo(approx);
        let (y, m, d) = JulianDay(shuo_jd + 1.0 / 3.0).to_ymd();
        items.push((y, m, d));
    }
    let leap = if items.len() > 13 {
        // 无中气的月份 = 闰月
        (1..items.len().saturating_sub(1)).find(|&i| {
            let start = JulianDay::from_ymd(items[i].0, items[i].1, items[i].2).0;
            let end = JulianDay::from_ymd(items[i + 1].0, items[i + 1].1, items[i + 1].2).0;
            (0..12).all(|q| {
                let tj = calc_qi(start, q as f64 * 30.0);
                tj < start || tj >= end
            })
        })
    } else {
        None
    };
    Some((items, leap, era))
}

use super::era_table::{ERA_TABLE, EraRecord};

/// 查询指定公历年份对应的年号/纪年记录
///
/// 返回 (年号记录引用, 该年在该年号中的序数)。
/// 如556年跨东魏+梁可能返回多条。
pub fn era_names(year: i32) -> Vec<(&'static EraRecord, u32)> {
    let mut results = Vec::new();
    for rec in ERA_TABLE {
        if year >= rec.start && year < rec.start + rec.duration {
            let ordinal = (year - rec.start + 1) as u32;
            results.push((rec, ordinal));
        }
    }
    results
}

/// 历史纪年 → 八字一条龙
///
/// 输入年号名、在位年、农历月日、时辰索引，推算公历日期并输出八字。
/// 经度缺省东八区（120°E），可通过 `longitude` 指定。
pub fn era_bazi(
    era_name: &str,
    ordinal: u32,
    lunar_month: u32,
    lunar_day: u32,
    is_leap: bool,
    hour_index: usize, // 时辰索引 0=子时...11=亥时
    longitude: f64,
) -> Option<crate::calendar::ganzhi::Bazi> {
    let records = find_era(era_name);
    if records.is_empty() {
        return None;
    }
    // 取第一个匹配的年号记录
    let rec = records[0];
    let solar_year = rec.start + ordinal as i32 - 1;

    let lunar = LunarDate::new(solar_year, lunar_month, lunar_day, is_leap);
    let (y, m, d) = lunar_to_solar(&lunar);
    let hour = (hour_index * 2) as u32;
    Some(bazi(y, m, d, hour, 0, longitude))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompress_sb() {
        assert_eq!(sb_table().len(), 16598, "SB 16598");
        assert_eq!(qb_table().len(), 7567, "QB 7567");
        for &b in sb_table().iter() {
            assert!(b == b'0' || b == b'1' || b == b'2');
        }
        for &b in qb_table().iter() {
            assert!(b == b'0' || b == b'1' || b == b'2');
        }
    }

    #[test]
    fn test_low_shuo_epoch() {
        // rough_shuo 是 5 项三角的简化公式
        // w=0 → 第 0 个朔日，返回值是 J2000 偏移 JD + 8/24
        // 第 0 个朔在 2000-01-06 18:14 UTC ≈ J2000 偏移 5.257 天
        let jd = rough_shuo(0.0);
        let diff = (jd - 2451550.257 + J2000).abs();
        assert!(
            diff < 1.0,
            "low_shuo at w=0 should be near epoch, diff={} d",
            diff
        );
    }

    #[test]
    fn test_low_qi_vernal() {
        let (_y, m, _d) = JulianDay(rough_qi(0.0) + J2000).to_ymd();
        assert_eq!(m, 3);
    }

    #[test]
    fn test_calc_shuo_fallback() {
        assert!(calc_shuo(MODERN_EPOCH_JD + 100.0) > MODERN_EPOCH_JD + 50.0);
    }

    #[test]
    fn test_items_1800() {
        let (moons, _, era) = shuo_list(1800).unwrap();
        assert_eq!(moons.len(), 15);
        assert_eq!(era, HistoricalEra::Standard);
        // 十一月 ≈ 12 月，年份一般同 year
        assert_eq!(
            moons[0].1, 12,
            "first moon month should be Dec, got {}",
            moons[0].1
        );
    }

    #[test]
    fn test_items_era() {
        // -500 在 SpringAutumn 范围 (-721 ~ -479)
        assert_eq!(shuo_list(-500).unwrap().2, HistoricalEra::SpringAutumn);
    }

    #[test]
    fn test_items_out() {
        assert!(shuo_list(1900).is_none());
        assert!(shuo_list(-800).is_none());
    }

    #[test]
    fn test_era_names_tang() {
        let n = era_names(627);
        assert!(n.iter().any(|(rec, ord)| rec.era == "贞观" && *ord == 1));
    }

    #[test]
    fn test_era_names_multi() {
        assert!(era_names(556).len() >= 2);
    }

    #[test]
    fn test_find_era_zhenguan() {
        let recs = crate::calendar::era_table::find_era("贞观");
        assert!(!recs.is_empty());
        assert_eq!(recs[0].start_year(), 627);
    }

    #[test]
    fn test_era_bazi_zhenguan() {
        let bazi = era_bazi("贞观", 3, 5, 3, false, 4, 120.0).unwrap();
        assert_eq!(bazi.year.gan, crate::calendar::ganzhi::Tiangan::Ji);
        assert_eq!(bazi.year.zhi, crate::calendar::ganzhi::Dizhi::Chou);
    }

    #[test]
    fn test_lunar_month_from_history() {
        assert_eq!(
            lunar_month_from_history(2, None, false, HistoricalEra::Standard).0,
            1
        );
        assert_eq!(
            lunar_month_from_history(2, None, false, HistoricalEra::SpringAutumn).0,
            2
        );
    }
}
