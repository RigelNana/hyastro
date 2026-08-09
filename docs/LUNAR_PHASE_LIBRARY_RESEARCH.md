# 月相与朔望事件库调研（LUNAR_PHASE_LIBRARY_RESEARCH）

- 调研日期：2026-08-09
- 调研基线版本：`sofars = 0.6.1`（crates.io 锁定 `=0.6.1`）、`anise = 0.10.4`（`default-features = false`）、`hifitime = 4.3.0`（`Cargo.lock` 锁定；registry 另存 4.3.1 未采用）
- 配套文档：`docs/PRD.md`（6.8.3 月相与天体配置）、`docs/FEATURES.md`（19.1 月相和季节）、`docs/DEPENDENCIES.md`、`docs/DOMAIN_MODEL.md`
- 结论性质约定：**直接可复用**（进 hyastro 生产路径，无需改写）、**仅底层/交叉验证**（供自研算法取数或差分，不直接进公开接口）、**不适合**（语义错位或依赖门控不允许）
- 证据标注：`[观察]` 直接读自源码/官方文档；`[推断]` 基于观察的推理。本地路径为 `/home/rigel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/<crate>/<path>:<行号>`，下文简写为 `<crate>/<path>:<行号>`。

---

## 0. 结论速览

### 0.1 三个待澄清问题的直接回答

| 问题 | 结论 | 一句话依据 |
|---|---|---|
| ANISE 的 `PhaseAngle` 是月相吗？ | **不是**。它是 SPICE PCK 行星极轴/首子午线的多项式定向角系数（`offset_deg`/`rate_deg`/`accel_deg`），仅用于行星定向（pole RA/Dec、prime meridian），与月球照明几何完全无关。 | `anise/src/structure/planetocentric/phaseangle.rs:18-55`；`anise/src/structure/planetocentric/mod.rs:77-84` |
| ANISE `sun_angle_deg` 是否足以区分四相？ | **不足**。它返回无符号地心距角（solar elongation，0–180°）：朔附近取极小、望附近取极大，上弦/下弦都落在 ≈90°；距角极值时刻又不等于黄经差为 0°/180° 的标准时刻，因此四相都不能直接由它定义。 | `anise/src/almanac/solar.rs:25-36,74-100` |
| sofars `moon98`/`epv00` 能否直接给出月相事件？ | **不能**。SOFA 只提供"某时刻月球/地球位置"，没有任何事件搜索、没有月相函数（SOFA 全集无"phase of the Moon"）；朔望时刻必须由 hyastro 自己组合（月球+太阳位置 → 黄经差 → 求根）。 | `sofars/src/eph/moon98.rs:21`、`sofars/src/eph/epv00.rs:6039`；`docs/LIBRARY_RESEARCH.md:103`（"无事件计算（升落/中天/月相）"） |

### 0.2 能力判定表（核心交付）

| 库/能力 | 月相中的角色 | 判定 |
|---|---|---|
| sofars `moon98` | 低精度月球地心位置（GCRS，TT 输入），RMS 2.9″/6.1 km vs ELP/MPP02（1950–2100） | **直接可复用**（低精度后端） |
| sofars `epv00` | 低精度太阳方向（日心地球向量），RMS 3.7 km vs DE405（1900–2100） | **直接可复用**（低精度后端） |
| sofars `plan94` | 行星位置（无月球、无地球） | **不适合**（月相） |
| SOFA 事件搜索/月相函数 | 不存在 | 必须自研组合 |
| ANISE SPK `translate` | DE440 高精度月球/太阳地心状态（几何 + 光行时 + 光行差） | **直接可复用**（高精度后端，经 hyastro `ephem::Ephemeris` 隔离） |
| ANISE `sun_angle_deg` | 距角量值（朔/望粗判） | **仅交叉验证**（无符号、角距≠黄经差） |
| ANISE `PhaseAngle` | PCK 定向系数 | **不适合**（命名陷阱） |
| ANISE `analysis`（`report_events` 等） | 事件求根引擎 | **不适合**（生产）：feature 门控、Orbit/航天器语义、S-表达式 DSL、类型泄漏；仅开发期差分参考 |
| hifitime 4.3.0 | 时间线/尺度（TAI/TT/TDB/ET/UTC…）、Duration、TimeSeries；SPK 求值时间轴 | **直接可复用**（时间底座，非月相算法） |
| rust-astro（astro crate） | Meeus 月相（`lunar.rs`） | **不适合**（依赖）；仅作公式参考（已见 `docs/LIBRARY_RESEARCH.md:325`） |

---

## 1. 语义先行：月相的判据定义（先定标准，再谈库）

### 1.1 四个主相的定义（采用 USNO/天文年历惯例）

朔（New Moon）、上弦（First Quarter）、望（Full Moon）、下弦（Last Quarter）定义为**地心视黄经差**到达 90° 整数倍的时刻：

$$D(t) = \lambda_M(t) - \lambda_S(t) \equiv 90^\circ \cdot k \pmod{360^\circ},\quad k = 0,1,2,3$$

其中 $\lambda_M$、$\lambda_S$ 是地心视位置（含光行时、周年光行差后）在**日期真黄道**（true ecliptic and equinox of date）上的黄经。这一定义与 Meeus《Astronomical Algorithms》第 49 章、NASA 月相目录（Espenak，基于 Meeus）一致；hyastro 的日期真黄道语义已在 `CONTEXT.md` 明确定义（IAU 2006 frame bias/岁差 + IAU 2000A 章动 + 真黄赤交角 $\epsilon_A+\Delta\epsilon$）。

### 1.2 照亮比例与月相角

月相角 $\alpha$（月球处的日地夹角，angle Sun–Moon–Earth at the Moon）与照亮比例：

$$k = \frac{1 + \cos\alpha}{2}, \qquad \cos\alpha = \frac{(\vec r_S - \vec r_M)\cdot(-\vec r_M)}{|\vec r_S - \vec r_M|\,|\vec r_M|}$$

其中 $\vec r_M$、$\vec r_S$ 是地心月球/太阳位置向量（Meeus 第 48 章）。按标准约定 $\alpha = 180^\circ$ 朔（新）、$0^\circ$ 望（满）、$\approx 90^\circ$ 弦。注意 $\alpha$ 与地心距角 $\psi$（elongation，ANISE `sun_angle_deg` 所给）是**互补**关系：$\alpha \approx 180^\circ - \psi$，差值即太阳相对地月基线（384400 km）的视差，最大约 0.147°——用 $180^\circ-\psi$ 代替 $\alpha$ 会给照亮比例 $k$ 引入最大约 $1.3\times10^{-3}$ 的误差 [推断]，因此照亮比例必须用 1.2 的向量公式直接求 $\alpha$，不能拿距角换算。**且 $\alpha$、$\psi$ 都是无符号量**，无法区分上弦/下弦——盈亏方向必须由 $D(t)$ 的符号（即 $\sin D$ 的符号）决定。

### 1.3 三个必须显式化的语义决策（影响事件时刻可达 ~40 s，大于历表误差）

| 决策点 | 量级影响 [推断] | 建议 |
|---|---|---|
| 几何 vs 视位置（光行时） | 月球光行时 1.28 s × 相对角速 0.508″/s ≈ 0.65″ → 朔望时刻差 ~1.3 s；太阳光行时 499 s × 0.041″/s ≈ 20.5″ → 若日/月不一致可达 ~40 s | 统一用**地心视位置**（hyastro `geocentric_apparent_place` 对日月同一条链，天然一致） |
| 黄经合 vs 最小角距 | 月球黄纬 ±5.3°，最小角距时刻与黄经差为零时刻相差数分钟量级 | 采用黄经差定义（USNO 惯例）；`sun_angle_deg` 不能直接当朔 |
| 上/下弦判定 | 距角恒为正（0–180°），弦相退化为同值 | 用带符号 $D(t)$（黄经差）求根 $D \equiv 90k$ |

---

## 2. sofars 0.6.1（IAU SOFA 纯 Rust 内核）

- 身份：crates.io `sofars = 0.6.1`（仓库 astro-xao/sofars），MIT + SOFA 使用条款；`sofars/src/lib.rs` 公开模块 `astro/cal/consts/coords/eph/erst/fundargs/projection/pnp/star/ts/vm`。
- 历表域（`sofars/src/eph/`）只有三个函数：`epv00`（地球）、`moon98`（月球）、`plan94`（行星）。SOFA 官方 C 库同样只覆盖这三者（`docs/LIBRARY_RESEARCH.md:95`）。

### 2.1 `moon98` — 月球位置（低精度路径的核心）

- 签名：`pub fn moon98(date1: f64, date2: f64) -> [[f64; 3]; 2]`（`sofars/src/eph/moon98.rs:21`；本地实现把 SOFA 的 Notes 精简了，完整 Notes 见官方 C 源）。
- 输入：TT 两段式儒略日；输出：GCRS 月球 p,v（au、au/d）。
- 算法：Meeus《Astronomical Algorithms》2nd ed. 的月球算法（ELP2000-82B 截断 + 附加项，60 项 L/R + 60 项 B 级数），完全解析微分求速度；**月球光行时对平黄经的修正被省略**（官方 Note 2）。[观察]
- 官方精度（vs ELP/MPP02，1950–2100，ERFA/SOFA C `eraMoon98` Notes 3）：
  - RMS：地心方向 2.9″、位置 6.1 km、速度 36 mm/s；
  - 最坏：18.3″、31.7 km、172 mm/s。
- 时间语义（官方 Note 4）：输入用 TDB 或 TT 无显著差别；**用 UT 会产生约 30″ 误差**（月球 0.5″/s）。输出 GCRS（与 J2000.0 平赤道/分点差 ≤ 23 mas，官方 Note 5）。

**月相事件时刻换算**：月球相对太阳角速 = 360°/29.5306 d ≈ 0.5079″/s。2.9″ RMS → 朔望时刻 RMS 约 **5.7 s**；18.3″ 最坏 → 约 **36 s**。[推断，基于官方精度数字]

### 2.2 `epv00` — 地球位置（太阳方向的低精度来源）

- 签名：`pub fn epv00(date1: f64, date2: f64) -> Option<([[f64; 3]; 2], [[f64; 3]; 2])>`（`sofars/src/eph/epv00.rs:6039`），返回日心/质心地球 p,v（BCRS，au、au/d）；日期超出 1900–2100 返回警告。
- 算法：VSOP2000 简化解（`sofars/src/eph/epv00.rs:6020-6023` Notes 3）。
- 官方精度（vs DE405，1900–2100，Notes 4）：日心位置 RMS 3.7 km / max 11.2 km；质心 RMS 4.6 km / max 13.4 km。DE406 对比显示：1800/2200 误差约翻倍，1500/2500 约 10 倍，1000/3000 约 60 倍。[观察]
- 对月相：3.7 km @ 1 au ≈ 0.0051″ → 朔望时刻影响 ~0.01 s，可忽略。[推断]

### 2.3 `plan94` — 行星位置（不适用于月相）

- `pub fn plan94(date1: f64, date2: f64, np: i32) -> Result<([[f64; 3]; 2], i32), i32>`（`sofars/src/eph/plan94.rs:30`）；np=1..8（水星..海王星，np=3 是 EMB），**不含月球、不含地球**。官方 Note 3 明确：要地球位置请用 `epv00`。精度：vs DE102（1800–2050）最大误差 EMB 6″/1000 km，木星 71″/76000 km；vs DE200（1960–2025）RMS EMB 2010 km。[观察]
- 月相不需要行星位置；`plan94` 的用途是合/冲/距角极值等行星事件，不在本调研范围。

### 2.4 SOFA 范围与月相相关的可复用件

- SOFA 全集没有月相、没有事件搜索（`docs/LIBRARY_RESEARCH.md:103`）。
- 月相组合所需的其他件在 sofars 中现成可用：时间尺度（`ts`，含 `dtdb` TDB−TT）、IAU 2006 岁差/IAU 2000A 章动（`pnp`、`fundargs`）、日期平/真黄道（`coords::ecm06/eceq06/eqec06`，`sofars/src/coords/`）、基本角（`fundargs`）、向量工具（`vm`）。这些是"直接可复用"的构建块，但**组合责任在 hyastro**。

---

## 3. ANISE 0.10.4（`default-features = false`）

### 3.1 feature 门控（hyastro 现状）

- ANISE `Cargo.toml`（发布版）：`default = ["metaload", "analysis"]`；`analysis = ["rayon", "serde-lexpr", "csv", "hyperdual"]`；`metaload = ["url", "ureq", "platform-dirs", "regex", "serde_dhall"]`；另有 `python`/`embed_ephem`/`validation`。[观察]
- hyastro `Cargo.toml`：`anise = { version = "=0.10.4", default-features = false, optional = true }` → **`analysis` 与 `metaload` 均未启用**。
- 未门控、始终可用的月相相关件：`Almanac::load`（本地文件，`anise/src/almanac/mod.rs:318`）、`translate`（`anise/src/ephemerides/translations.rs:52`，`impl Almanac` L29）、`Aberration`（`anise/src/astro/aberration.rs:53`）、`sun_angle_deg`（`anise/src/almanac/solar.rs:74`）、帧常量 `SUN_J2000`/`MOON_J2000`/`EARTH_J2000`（`anise/src/constants.rs:398-400`）、天体 ID `SUN=10`/`MOON=301`/`EARTH=399`（`anise/src/constants.rs:32-34`）。
- `analysis` 模块由 `#[cfg(feature = "analysis")]` 门控（`anise/src/lib.rs:22`）。

### 3.2 SPK 状态与光行时/光行差语义

`translate(target_frame, observer_frame, epoch, ab_corr) -> Result<CartesianState, EphemerisError>`（`anise/src/ephemerides/translations.rs:52`）：

1. 无修正：纯几何，沿历表树求差（`translations.rs:89-135`）。
2. 带修正（SPICE `spkapo` 重写，`translations.rs:140-191`）：以 SSB 为中介求相对向量 → 单程光行时 $\Delta t = |r|/c$（$c = 299792.458$ km/s，`anise/src/constants.rs:12`）→ 目标在 $t - \Delta t$（接收模式）重取状态 → **不收敛** 1 次迭代（`LT`），**收敛** 3 次（`CN`）；再按需加恒星光行差（`stellar_aberration`，`anise/src/astro/aberration.rs`）。
3. `Aberration` 常量：`NONE`/`LT`/`LT+S`/`CN`/`CN+S`/`XLT`/`XLT+S`/`XCN`/`XCN+S`（接收/发射 × 收敛/不收敛 × 恒星）——与 SPICE `abcorr` 约定一致（`aberration.rs:64-110`）。
4. SPK 段内插值统一用 hifitime **ET 秒**：`epoch.to_et_seconds()`（`anise/src/naif/daf/datatypes/chebyshev.rs:58,201`、`hermite.rs:200-208`、`lagrange.rs:138`、`modified_diff.rs:127`；段边界 `naif/spk/summary.rs:210`）。ET 与严格 TDB 的差异（hifitime 注明"SPICE 约定的 ET 与真 TDB 略有不同"，`hifitime-4.3.0/src/timescale/mod.rs:95-97`）对月相在亚毫秒量级，可忽略。[推断]

**月相含义**：`translate(MOON_J2000, EARTH_J2000, t, Some(Aberration::CN))` 给出接收模式的月球**视方向**（发射时刻重取），是"地心视位置"所需的几何输入；对太阳同理。这与 hyastro 现有 `ephem::Ephemeris::state`（几何，`src/ephem/anise.rs`）互补——`state` 提供几何状态，光行时收敛在 `astro::astrometry` 层已实现（`geocentric_apparent_place`，`src/astro/astrometry.rs:1357`），hyastro **不需要** ANISE 的光行时路径进入公开接口，只需其几何状态。

### 3.3 `sun_angle_deg` 的精确语义与局限

`pub fn sun_angle_deg(&self, target_id: NaifId, observer_id: NaifId, epoch: Epoch, ab_corr: Option<Aberration>) -> Result<f64, EphemerisError>`（`anise/src/almanac/solar.rs:74`）：观测者→太阳单位向量与观测者→目标单位向量的 arccos，返回 0–180°。文档字符串自称这是"太阳距角（solar elongation）"并注明 0°≈新月、180°≈满月（`solar.rs:25-36`）。

局限（本调研核心结论）：
1. **无符号**：上弦（$\Delta\lambda=+90°$）与下弦（$\Delta\lambda=270°$）时距角都落在 ≈90°（月球黄纬只带来二阶小量），同一函数值对应两个不同的弦相；`sun_angle_deg == 90` 的根在每个月出现两次且分属上/下弦，无法用单一方程区分。
2. **角距极值 ≠ 黄经合**：朔/望的"最小角距"时刻与黄经差 $D \equiv 0/180$ 时刻因月球黄纬不同步，差可达数分钟量级，不能用作朔的定义。[推断]
3. 对 `observer_id = EARTH`、`target_id = MOON` 调用时，`translate` 已含光行时选项（`Some(Aberration::LT/CN)` 为视方向）。

因此 `sun_angle_deg` 只适合做开发期粗校验（新/满月时刻的独立口径），生产判据用黄经差。

### 3.4 `analysis` 事件求根（为何不采用）

- 能力面（`anise/src/analysis/`）：`Event` + `Condition`（Equals/Between/LessThan/GreaterThan/Minimum/Maximum，`analysis/event.rs:36-50`）、`ScalarExpr`（常量/向量分量/角/模/Atan2/Modulo/太阳距角 `SunAngle`/食分 `SolarEclipsePercentage` 等，`analysis/expr.rs:31`）、`StateSpec`/`StateSpecTrait`（`analysis/specs.rs:134,186`）、`report_events`/`report_event_arcs`（`analysis/search.rs:45,202`）＝自适应步扫描 + Brent 求根（`analysis/utils.rs:128,18`），`epoch_precision` 默认 10 ms。
- ANISE 的内建 `ScalarExpr::SunAngle` 只能搜索无符号距角，不能表达 USNO 的四相定义；现有 DSL 也没有“同一历元的日、月日期真黄经差”这一直接标量。即使另写 `StateSpecTrait` 或扩展 DSL，仍然只是把 hyastro 已有的视位置与事件接缝重新实现一遍。
- **不采用理由**（生产路径）：
  1. 需要启用 `analysis` feature（新增 rayon/serde-lexpr/csv/hyperdual 依赖），与 hyastro 最小化默认构建冲突；
  2. API 面向航天器轨道语义（S-表达式 DSL、serde 序列化事件、`EventDetails`/`VisibilityArc`），与 hyastro 的 `EventEvidence`/`TimeInterval` 强类型事件模型错位；
  3. hyastro 公开接口禁止泄漏 ANISE 类型（`docs/DEPENDENCIES.md` 0.2 原则 4），把事件引擎建立在 ANISE 类型上会整层泄漏；
  4. hyastro 已有同构且更贴合事件的求根引擎（`src/event/search.rs` 的 `AngularEventSearchOptions` + `BracketedRootSearch`，被 `solar_terms_in` 使用）。
- 保留用途：只把 ANISE 的几何状态结果用于开发期差分；不把 `analysis` 事件结果当作月相 oracle。

### 3.5 数据依赖

- 高精度路径必须有 SPK：hyastro 本地已放 `data/ephem/de440.bsp`（114.3 MB，全版 1550–2650，JPL 官方 `de440_and_de441.pdf`）。`de440s.bsp`（缩短覆盖版，约 1849–2150 [推断，按 NAIF 通用内核惯例]）可在测试环境另备。
- 覆盖区间查询已由 `ephem::Ephemeris::coverage` 提供（`src/ephem/anise.rs`），区间外查询报错不静默外推——与 PRD EPH-API-005 一致。

---

## 4. hifitime 4.3.x：时间底座（非月相算法）

- 角色：ANISE 的硬依赖（`anise/Cargo.toml`：`hifitime = "4.3.0"`）；hyastro 的时间适配层（`src/time/hifitime.rs`）以 `HifitimeScale` trait 把 TAI/TT/TDB/TCG/TCB/GPST 映射到 hifitime `TimeScale`（`hifitime-4.3.0/src/timescale/mod.rs:90` 枚举含 TAI/TT/ET/TDB/UTC/GPST/GST/BDT/QZSST/TCG/TCB/TL/TCL），`export::<Tdb>` 产出 ANISE 查询历元（`src/ephem/anise.rs` 中 `Hifitime::new().export(query.epoch().retag::<Tdb>())`）。
- 月相相关的时间能力：
  - `Epoch::to_tdb_duration()`（`hifitime-4.3.0/src/epoch/mod.rs:923`）——ANISE `PhaseAngle` 求值用（`anise/.../phaseangle.rs:52`）；TDB 参考常数 `TDB0_S = -6.55e-5`（`hifitime-4.3.0/src/epoch/mod.rs:88`）。
  - `Epoch::to_et_seconds()`/`to_et_duration()`（`epoch/mod.rs:860,874`）——SPK 插值时间轴（见 3.2）。
  - `TimeSeries::inclusive/exclusive`（`hifitime-4.3.0/src/timeseries.rs:108,63`）——可做粗扫描迭代器；但**求根/括根在 hyastro**（`src/event/search.rs`），hifitime 不提供。
  - hyastro 的 UTC 解析走自己的 `TimeContext`（闰秒表），刻意不经过 hifitime UTC（`src/time/hifitime.rs:8-11`）。
- 结论：hifitime 无任何月相/天文事件算法；它的作用是物理时间线、尺度转换（TT→TDB→ET）、Duration/TimeSeries，为两个历表后端（SOFA 的两段式 JD、ANISE 的 ET 秒）提供同一套强类型时间输入。

---

## 5. 不采用方案（至少一个，附完整理由）

| 候选 | 判定 | 理由 |
|---|---|---|
| ANISE `PhaseAngle` 当作月相 | **不适合** | 概念错误：PCK 定向角系数（`phaseangle.rs:18-55`），用于 pole RA/Dec/prime meridian（`planetocentric/mod.rs:77-84`）；与照明几何无关。命名极具误导性，必须在本文档与代码注释中显式辟谣 |
| 仅用 `sun_angle_deg` 定义四相 | **不适合** | 无符号距角无法区分上/下弦；角距极值≠黄经合（见 1.3、3.3） |
| ANISE `analysis`/`report_events` 作为生产事件引擎 | **不适合**（仅开发期差分） | feature 门控（3.1）、Orbit/航天器语义、S-表达式 DSL、公开类型泄漏、与 `src/event/search.rs` 重复（3.4） |
| rust-astro（astro crate）直接依赖做月相 | **不适合**（既有决策） | 2018 年停更、edition 2015；月相函数实际位于 `src/lunar.rs:990-1195`，公开注释只声明 1980–2020 中期的平均误差约 3.8 s，且实现中弦相修正 `W` 的末项被分号截断，不能作为可信生产依赖 |
| Meeus 第 49 章相位公式作为主生产路径 | **不适合**（仅公式参考/初值） | 它直接预测事件时刻，不经过项目统一的 DE440 视位置链，也不保留历表覆盖、光行时语义和求根证据；NASA 六千年月相目录可作到分发布值 oracle，但不能替代同一判据上的 DE440 求根 |

---

## 6. 推荐实现架构与数据流

### 6.1 数据流（mermaid）

```mermaid
flowchart LR
    A[时间输入 Instant&lt;S&gt;] --> B[TimeContext 尺度模型]
    B --> C1[ANISE DE440 几何状态]
    B -. 未来低精度层 .-> C2[sofars moon98+epv00]
    C1 --> D1[现有 Astrometry::geocentric_apparent_place 日/月]
    C2 --> D2[私有解析视位置 evaluator]
    D1 --> E[日期真黄道黄经 λ_M, λ_S]
    D2 --> E
    E --> F[D = λ_M − λ_S 连续化 unwrap]
    F --> G[扫描括根 + Brent 精化<br/>复用 AngularEventSearchOptions/BracketedRootSearch]
    G --> H[LunarPhaseEvent：相位/时刻/证据]
    D --> I[月相角 α 与照亮比例 k=(1+cosα)/2]
    I --> H
    H --> J[验证：NASA 月相目录 / JPL Horizons / 双后端差分]
```

### 6.2 模块边界建议

- **实现状态**：`src/astro/lunar.rs` 已公开强类型 `MoonPhaseAngle` 和 `LunarIllumination<S>`；后者组合月—地、日—地、日—月三条收敛光行时并返回视距角、有向视黄经差、物理相位角、球形月面照亮比例和 `MoonPhaseBranch`。`src/event/lunar_phase.rs` 已公开任意角过境、`MoonPhase`/`MoonPhaseEvent<S>` 四相包装、区间搜索和固定偏移公历年结果。瞬时物理量不绑入事件结果，四个主相位与任意角搜索则共享同一个 `MoonPhaseAngleEvent<S>` 精化内核。
- **复用现有接缝，不新建抽象**：
  - `Astrometry::geocentric_apparent_place(target: CelestialBody, ...)`（`src/astro/astrometry.rs:1357`）已对**任意有限目标**（含 `CelestialBody::Moon`）实现“接收历元固定 → 目标发射时刻迭代 → 自然视线 → 有限距离太阳单极偏折 → 周年光行差 → GCRS/日期真赤道/日期真黄道”完整链，返回含 `true_ecliptic()`/`longitude()`/`distance()` 的 `GeocentricApparentPlace`；太阳已有专用视图 `solar_apparent_place`（`astrometry.rs:1402`，`src/astro/solar.rs`）。月相只需调用两次（日、月）并取 `true_ecliptic` 黄经差。
  - 事件骨架：`src/event/search.rs` 的 `AngularEventSearchOptions`（扫描步/时间容差/角容差/Brent 上限/求值上限）+ `BracketedRootSearch::refine`（二者均在 `src/event/search.rs`）；`solar_terms_in`（`src/event/solar_term.rs:325`）是“连续周期角求根”的样板，月相是其直接推广（判据从“太阳黄经=15°k”换成“日月黄经差=90°k”）。
  - 后端：首版只走现有具体类型 `ephem::Ephemeris`（ANISE DE440）。当前 `Astrometry` 直接持有 `&Ephemeris`，并不存在可替换的历表 trait；若以后确有无 SPK 的低精度产品需求，再先提取后端接缝，或增加私有的 `moon98+epv00` evaluator。不要为了首版月相预先抽象。
- **语义固定**（写进类型文档与测试）：判据=地心视黄经差（日期真黄道）；光行时=接收模式收敛；`waxing/waning` 由 $D$ 的符号变化方向判定；"最小角距"不参与朔望定义（可另作 F-WIN-003 最小月距的独立工作流）。

---

## 7. 精度分层与验证基准

### 7.1 精度分层

| 层 | 后端 | 朔望时刻不确定度预算 | 用途 |
|---|---|---|---|
| L0 民历级 | sofars `moon98`+`epv00` | 若只取同历元几何向量，会另带约 40 s 的视位置语义偏差；补齐一致的光行时/光行差后，历表方向误差折算为 RMS ~6 s、最坏 ~40 s [推断] | 快速/教学/日历级（F-MOON-002 “明确误差范围的快速月球模型”） |
| L1 天文级 | ANISE DE440（`data/ephem/de440.bsp`，1550–2650） | 历表贡献 ≪1 s（DE440 月球由 LLR 约束、米级；插值段内切比雪夫误差同量级）[推断]；实际主导项是判据/语义一致性（见 1.3） | 生产默认 |
| L2 外部核验 | JPL Horizons（DE441）向量差分 + USNO/NASA 发布表 | 求根数值误差可压到毫秒；DE440/DE441 与发布值的实际差异必须测量后再定阈值，USNO/NASA 表本身只有分钟分辨率 | CI 验证 |

### 7.2 验证来源（可点击一手来源）

- **NASA Six Millennium Catalog of Phases of the Moon**（Espenak，基于 Meeus 第 49 章，UTC 到分，含 ΔT 列）：https://eclipse.gsfc.nasa.gov/phase/phasecat.html —— 民历级 oracle（注意：其算法是 Meeus 而非 DE440）。
- **JPL Horizons**（DE441，任意时刻日月黄经差/距角查询，可输出朔望表）：https://ssd.jpl.nasa.gov/horizons/app.html —— L1 层 oracle。
- **USNO 月相数据**：https://aa.usno.navy.mil/data/MoonPhases —— 官方发布值。
- **DE440 官方论文**：Park et al. 2021, *The JPL Planetary and Lunar Ephemerides DE440 and DE441*, AJ 161:105（DOI 10.3847/1538-3881/abd414；NAIF 镜像 PDF：https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440_and_de441.pdf）——覆盖范围 1550–2650、月球轨道由 LLR 定轨、ICRS 定向 ≤0.2 mas。
- **SOFA 官方函数说明**：https://www.iausofa.org/current_C.html（epv00/moon98/plan94 的 Notes 与精度表）；本调研同时核对 ERFA 镜像 C 源（`liberfa/erfa` 的 `src/moon98.c`/`plan94.c`/`epv00.c`，与 SOFA 同源、Notes 完整）。
- **Meeus**：《Astronomical Algorithms》2nd ed.，Willmann-Bell, 1998，第 48/49 章（相位角、照亮比例与主相解析公式；精度必须按目标年代和定义口径实测）。

### 7.3 测试形态建议（沿用既有模式）

1. **发布值到分**：仿 `tests/solar_term_contracts.rs`（`de440s_2024_solar_terms_match_hong_kong_observatory_minutes`，发布值±分钟断言 + `HYASTRO_DE440S` 环境变量指定内核）——用 NASA phasecat（2001–2100 表）或 HKO 日历的朔望/节气表做 L1 层分钟级断言。
2. **双后端差分**：同一判据（$D \equiv 90k$）分别用 DE440 与 `moon98+epv00` 求根，断言时间差 ≤ 60 s（RMS ~6 s 预算，留裕量）。
3. **求根契约**：单事件残差 ≤ 角容差（picorad 级）、`EventEvidence` 括区间/迭代/求值计数非退化；对 `D` 的符号连续性（unwrap）做属性测试（proptest）。
4. **SOFA 官方数值**：`moon98`/`epv00` 的 SOFA 官方 t_sofa_c 用例已由 sofars 自带测试复刻（`docs/LIBRARY_RESEARCH.md:97`），hyastro 接缝层只需差分测试，无需重复官方值。

---

## 8. 证据表（关键结论 → 证据 → 来源）

| # | 结论 | 证据 | 来源 |
|---|---|---|---|
| 1 | ANISE `PhaseAngle` 是 PCK 定向角系数而非月相 | `pub struct PhaseAngle<const N: usize> { offset_deg, rate_deg, accel_deg }`；`evaluate_deg(epoch, rate_unit)` 用 `epoch.to_tdb_duration()`；用于 `pole_right_ascension`/`pole_declination`/`prime_meridian` | `anise/src/structure/planetocentric/phaseangle.rs:18-55`、`planetocentric/mod.rs:77-84`；仓库 https://github.com/nyx-space/anise |
| 2 | `sun_angle_deg` 返回无符号距角（0–180°），文档自述 0°=新月/180°=满月，但无法区分上/下弦 | "Returns the angular separation (between 0 and 180 degrees)... This is formally known as the solar elongation... ~0° (Conjunction)... ~180° (Opposition)" | `anise/src/almanac/solar.rs:25-36,74-100`；docs https://nyxspace.com/ |
| 3 | SOFA 无月相/事件函数，`moon98`/`epv00` 只给位置 | 历表模块仅 `epv00`/`moon98`/`plan94` 三个函数；项目调研早已记录"无事件计算（升落/中天/月相）" | `sofars/src/eph/*.rs`；`docs/LIBRARY_RESEARCH.md:103`；SOFA 官方 https://www.iausofa.org/current_C.html |
| 4 | `moon98` 精度：vs ELP/MPP02（1950–2100）RMS 2.9″/6.1 km/36 mm/s，最坏 18.3″/31.7 km/172 mm/s | `eraMoon98` Notes 3（ERFA C 源，与 SOFA 同源） | https://raw.githubusercontent.com/liberfa/erfa/master/src/moon98.c（Notes 1-7）；sofars 本地 `src/eph/moon98.rs:21` |
| 5 | `epv00` 精度：vs DE405（1900–2100）日心位置 RMS 3.7 km/max 11.2 km；范围外误差膨胀 | `eraEpv00` Notes 4（sofars 内嵌完整 Notes） | `sofars/src/eph/epv00.rs:6018-6035`；https://raw.githubusercontent.com/liberfa/erfa/master/src/epv00.c |
| 6 | `plan94` 不含月球与地球（np=3 为 EMB），不适用于月相 | `eraPlan94` Notes 3、np=1..8 定义 | `sofars/src/eph/plan94.rs:17-21,30`；https://raw.githubusercontent.com/liberfa/erfa/master/src/plan94.c |
| 7 | ANISE `translate` 光行时：不收敛 1 次 / 收敛 3 次迭代，接收/发射模式，恒星可选；SPK 插值用 hifitime ET 秒 | `translations.rs:142-191`（`num_it = if converged {3} else {1}`）；`chebyshev.rs:58,201`、`hermite.rs:200-208`、`lagrange.rs:138` | `anise/src/ephemerides/translations.rs`、`anise/src/naif/daf/datatypes/*.rs`、`anise/src/naif/spk/summary.rs:210` |
| 8 | ANISE `analysis` 被 feature 门控，默认关闭；hyastro `default-features=false` 未启用 | `analysis = ["rayon","serde-lexpr","csv","hyperdual"]`；`#[cfg(feature = "analysis")] pub mod analysis` | `anise/Cargo.toml`、`anise/src/lib.rs:22`；hyastro `Cargo.toml` |
| 9 | ANISE 事件引擎：`report_events`=自适应步扫描+Brent；`Orbit` 即 `CartesianState` 别名 | `analysis/search.rs:45,202`；`analysis/utils.rs:18,128`；`astro/orbit.rs:46` | 同上 |
| 10 | hifitime 无月相算法，提供 TAI/TT/ET/TDB/UTC 等尺度、`to_et_seconds`/`to_tdb_duration`、TimeSeries | `TimeScale` 枚举、`Epoch` 方法、`TimeSeries::inclusive/exclusive` | `hifitime-4.3.0/src/timescale/mod.rs:90`、`epoch/mod.rs:860,923`、`timeseries.rs:39,63,108` |
| 11 | hyastro 现有接缝可支撑月相：`geocentric_apparent_place` 通用有限目标；`solar_terms_in` 是周期黄经求根样板 | `geocentric_apparent_place`（target: CelestialBody）返回含 `true_ecliptic`/`longitude` 的 `GeocentricApparentPlace`；`AngularEventSearchOptions`/`BracketedRootSearch` | `src/astro/astrometry.rs:1357`、`src/astro/solar.rs`、`src/event/solar_term.rs:325`、`src/event/search.rs` |
| 12 | DE440 覆盖 1550–2650；DE440 月球轨道由 LLR 定轨 | 官方论文："DE440 covering years 1550–2650"、"The orbit of the Moon is determined from laser ranging to lunar retroreflectors" | Park et al. 2021（DOI 10.3847/1538-3881/abd414）；NAIF PDF：https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440_and_de441.pdf |
| 13 | NASA 月相目录（民历级 oracle）基于 Meeus 第 49 章，UTC 到分，含 ΔT | 页面正文："Algorithms used in predicting the phases of the Moon... based on Jean Meeus' Astronomical Algorithms (1998)" | https://eclipse.gsfc.nasa.gov/phase/phasecat.html |

---

## 9. 参考来源汇总

- sofars 0.6.1：https://crates.io/crates/sofars 、 https://github.com/astro-xao/sofars 、本地 `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sofars-0.6.1/`
- SOFA 官方：https://www.iausofa.org/ 、 https://www.iausofa.org/current_C.html
- ERFA（SOFA 同源镜像，Notes 完整）：https://github.com/liberfa/erfa （`src/moon98.c`、`src/epv00.c`、`src/plan94.c`）
- ANISE 0.10.4：https://github.com/nyx-space/anise 、 https://nyxspace.com/ 、 https://docs.rs/anise 、本地 `anise-0.10.4/`
- hifitime 4.3.0：https://github.com/nyx-space/hifitime 、 https://docs.rs/hifitime 、本地 `hifitime-4.3.0/`
- DE440/DE441：Park et al. 2021, AJ 161:105（https://doi.org/10.3847/1538-3881/abd414 ）；NAIF 内核 https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440.bsp
- 验证 oracle：https://eclipse.gsfc.nasa.gov/phase/phasecat.html 、 https://ssd.jpl.nasa.gov/horizons/app.html 、 https://aa.usno.navy.mil/data/MoonPhases
- Meeus, *Astronomical Algorithms*, 2nd ed., 1998（第 48、49 章）
- hyastro 内部：`docs/PRD.md`（EVT-CFG-001..007）、`docs/FEATURES.md`（F-PHASE-001..007）、`docs/DOMAIN_MODEL.md`、`docs/DEPENDENCIES.md`、`src/astro/astrometry.rs`、`src/event/{search,solar_term}.rs`、`src/ephem/anise.rs`、`src/time/hifitime.rs`
