# hyastro

[![Rust 1.89+](https://img.shields.io/badge/rust-1.89%2B-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](Cargo.toml)

`hyastro` 是一个强调**物理语义、数据来源与数值证据**的强类型 Rust 天文算法库。

它不把时间尺度、参考系、原点、历元、单位或大气折射压缩成没有语义的裸 `f64`；需要 JPL 星历或 IERS 地球定向数据的工作流，也不会静默下载数据、越过覆盖范围或替调用者选择接纳策略。

> 当前版本为 `0.1.0`，公开接口仍可能演进。适合研究、验证和可复现计算；用于生产或观测计划前，请同时评估星历、EOP、大气、地形和仪器误差。

## 设计原则

- **强类型边界**：角度、长度、速度、时间尺度、历法、坐标轴、原点和历元均有独立语义。
- **显式模型**：IAU/SOFA 模型、JPL BSP、IERS EOP、参考椭球和折射条件由类型或参数明确给出。
- **数据可追溯**：数据版本、覆盖范围、观测/最终/预报来源和接纳策略不会被隐藏。
- **数值证据可查询**：事件结果可保留最终括区间、时间误差、角残差、迭代次数和求值次数。
- **无静默降级**：数据缺失、超出覆盖、模型不适用和有损后端回退均返回明确错误。
- **核心可 `no_std`**：默认启用 `std`；基础数学、时间和类型化模型可按功能裁剪。

## 当前能力

| 模块 | 能力 |
| --- | --- |
| `math` | 强类型角度与物理量、球面几何、向量、矩阵、旋转、四元数、Brent 求根 |
| `time` | 公历/儒略历、两段式 JD/MJD、UTC/TAI/TT/UT1/TDB 等时间尺度、闰秒、IERS C04 与 finals2000A 解析 |
| `earth` | WGS 84 大地坐标、固定站点、ITRS/GCRS 状态和地球姿态数据 |
| `frame` | ICRS/BCRS/GCRS/CIRS/ITRS、黄道与银道方向、IAU 2006/2000A 地球定向链 |
| `ephem` | 统一 `EphemerisProvider` 接口、强类型相对状态与覆盖；默认 SOFA 解析后端，可选 ANISE/JPL BSP 后端 |
| `astro` | 光行时、站心视差、太阳偏折、相对论光行差、真空/折射观测位置、太阳时、球形视盘与月球测光 |
| `event` | 升中落、球形盘面地平接触、晨昏蒙影、节气、月相、行星配置、多类极值事件；固定站点地方日食 C1—C4；全球 Gamma、偏/环/全/全环食和非中心食分类、中心路径时间区间；显式物理或 NASA `k1/k2` 半径约定的贝塞尔即时根数、六小时多项式、解析导数和拟合残差 |
| `catalog` | 星表自行、有限距离空间运动、SOFA `starpm`、六参数协方差与数值 Jacobian 传播 |
| `uncertainty` | 强类型一倍标准不确定度、相关矩阵、来源标记与保守插值传播 |

完整范围与当前覆盖见 [`docs/FEATURES.md`](docs/FEATURES.md)。该目录同时列出尚未实现的高级能力；README 不把规划项描述为现有功能。

## 安装

从当前 Git 仓库使用默认 `std` 功能：

```toml
[dependencies]
hyastro = { git = "https://github.com/RigelNana/hyastro.git" }
```

启用 JPL BSP 后端和民用时间适配器：

```toml
[dependencies]
hyastro = {
    git = "https://github.com/RigelNana/hyastro.git",
    features = ["anise", "jiff"],
}
```

### Cargo features

| Feature | 默认 | 说明 |
| --- | :---: | --- |
| `std` | 是 | 启用标准库、SOFA 解析星历/天测工作流和 Hifitime 时间转换 |
| `anise` | 否 | 启用高精度 ANISE/JPL BSP 星历后端；同时启用 `std` |
| `hifitime` | 是（随 `std`） | 启用 `hifitime` 时间适配；也可在裁剪配置中单独选择 |
| `jiff` | 否 | 启用 `jiff` 民用时间适配 |

检查核心 `no_std` 配置：

```bash
cargo check --no-default-features
```

## 快速开始

基础数学接口不需要外部数据：

```rust
use hyastro::math::{Angle, HourAngle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let angle = Angle::from_degrees(30.0)?;
    let hour_angle = HourAngle::wrap_hours(-1.5)?;

    assert!((angle.sin().value() - 0.5).abs() < 1.0e-15);
    assert!((hour_angle.as_hours() - 22.5).abs() < 1.0e-14);
    Ok(())
}
```

运行对应示例：

```bash
cargo run --example math_angles_sphere
cargo run --example math_rotations_roots
cargo run --example time_core
cargo run --example analytic_solar_position
cargo run --example analytic_planetary_events
cargo run --example analytic_local_solar_eclipse
cargo run --example analytic_global_solar_eclipses
cargo run --all-features --example analytic_global_solar_eclipses -- data/ephem/de440.bsp
cargo run --example analytic_lunar_eclipses
cargo run --all-features --example analytic_lunar_eclipses -- data/ephem/de440.bsp
```

这些示例默认使用 `SofaAnalyticEphemeris`，无需 BSP 或网络：`analytic_solar_position` 计算地心太阳视位置、光行时和日期真赤道坐标；`analytic_planetary_events` 搜索 2024 年水星合日/大距/留以及木星冲日/距离极小；`analytic_local_solar_eclipse` 使用仓库中的 IERS C04 快照计算 2024-04-08 Dallas 地方日全食的 C1—C4、食甚、食分、遮掩比例、太阳高度和接触位置角；`analytic_global_solar_eclipses` 搜索 2026 年全球日食，输出 Gamma、偏/环/全/全环食分类、中心轴时间区间、全环食转换、六小时贝塞尔多项式和地理路径；`analytic_lunar_eclipses` 搜索 2022 年全球月食，输出半影/偏食/全食分类、P1/U1/U2/食甚/U3/U4/P4、两种食分、位置角、阶段持续时间，并用 IERS C04 计算 Dallas 的月出月落裁剪、月球高度、低空提示和曙暮光背景。带 BSP 参数的命令显式切换到 ANISE/DE440 后端；没有参数时仍使用解析历表。所有结果保留模型、数据快照和历表来源。

## 星历后端分层

`Astrometry` 和 `Events` 只依赖小型 `EphemerisProvider` 接口：几何 BCRS 状态、连续覆盖和结果溯源。后端由调用者显式构造，库不会在精度不足、目标缺失或覆盖外时自动切换模型。

| 后端 | Feature / 数据 | 覆盖与用途 |
| --- | --- | --- |
| `SofaAnalyticEphemeris` | 默认 `std`；无外部文件 | SSB、太阳、地球、月球，以及 PLAN94 的水星至海王星系统质心。日地 `epv00` 为 1900–2100，月球 `moon98` 为 1950–2100；仅含 PLAN94 系统与太阳的查询为 1000–3000，与地球/月球/SSB 组合时取模型覆盖交集。适合快速近似、教学、初筛和无内核测试；逐系统 PLAN94 误差统计由 `Plan94Accuracy` 公开。 |
| `Ephemeris`（ANISE） | `anise`；调用者提供 BSP | 高精度 JPL/SPK 状态与内核实际支持的目标/中心链。精度、目标和覆盖由所选内核决定，适合生产级结果与更广太阳系目标。 |

两个后端返回同一 `RelativeState<Bcrs, S>`、`Coverage<Bcrs, S>` 和稳定错误分类。解析后端不伪装成 JPL 精度；ANISE 后端也不会隐式下载或猜测内核。

## 高精度 JPL 星历与 IERS EOP

`hyastro` 不在构建期或运行时自动下载天文数据。

- JPL/NAIF BSP 由调用者提供，并通过 `KernelManifest::inspect` 显式检查；仓库的 `data/ephem/*.bsp` 路径被 Git 忽略。
- IERS C04 与 finals2000A 快照位于 [`data/eop`](data/eop)，来源记录在 [`data/eop/SOURCES.toml`](data/eop/SOURCES.toml)。
- EOP 使用者必须选择 `FinalOnly`、`ObservedOrFinal` 或 `IncludePredicted`；缺列和覆盖空洞不会被当成数值零。
- 计算结果只在所选 BSP、EOP、闰秒表和模型覆盖范围内成立。

使用本地 DE440 系列 BSP：

```bash
cargo run --features anise --example ephemeris_de440s -- /path/to/de440s.bsp
```

完整的当前太阳位置工作流还要求显式 EOP、站点和气象输入：

```bash
cargo run --release --features anise,jiff \
  --example current_solar_position -- \
  /path/to/de440s.bsp \
  /path/to/finals2000A.all \
  31.340370 121.458917 15.0 \
  1013.25 15.0 0.65 0.55
```

## 实时天文钟

终端示例组合了 JPL DE440、IERS EOP、WGS 84 站点、太阳/月球天测、节气、行星位置和事件求根：

```bash
cargo run --release --features anise,jiff \
  --example astronomical_clock -- /path/to/de440.bsp
```

只输出一次快照：

```bash
cargo run --release --features anise,jiff \
  --example astronomical_clock -- /path/to/de440.bsp --plain
```

该示例默认展示一个固定 UTC+8 站点。代码中的经纬度、椭球高、气象参数、EOP 快照和 BSP 路径都是示例输入，不是库级全局默认值。

### 升落与“日出”定义

“太阳升起”不是脱离判据的单一时刻。示例并列展示：

1. **折射前（真空）**：上缘、中心和下缘分别穿越天文地平线；
2. **标准地平折射**：采用固定 `34′` 折射量，并使用随 JPL 站心距离动态计算的球形视半径；
3. 升起按上缘 → 中心 → 下缘排序，落下按下缘 → 中心 → 上缘排序。

常见软件的标准日出通常接近“太阳中心真空高度 `-0.833°`”，即约 `34′` 标准折射加约 `16′` 太阳视半径。实际可见时刻还受实时气象、观测波段、海拔、地形地平线和太阳非球形细节影响。

## 精度与不确定度

强类型防止语义混用，但不自动产生完整误差预算：

- `EventEvidence` 中的 `±` 是最终求根括区间半宽；`|Δf|` 或 `|Δλ|` 是判据残差。
- 数值求根误差不等于 JPL 星历、EOP、大气模型、站点坐标或模型差异的物理不确定度。
- 默认 SOFA 解析后端的公里级模型误差和角秒级月球方向误差不包含在事件求根残差中；需要更高精度时应显式换用合适的 JPL BSP。
- `StandardUncertainty<Q>` 只表示与 `Q` 同量纲的一倍标准不确定度，不暗示误差独立、Gaussian 或协方差完整。
- 显示为 `σ—` 的量没有在当前工作流中完成物理/模型误差传播。

需要可审计结果时，应随输出保存：输入数据版本、覆盖范围、时间尺度、参考系、站点、气象条件、判据、搜索选项和数值证据。

## 更多示例

```bash
cargo run --example public_constants
cargo run --example math_quantities_vectors
cargo run --example time_eop
cargo run --example frames_earth_orientation
cargo run --example earth_site
cargo run --example analytic_solar_position
cargo run --features anise --example solar_apparent_position -- /path/to/de440s.bsp
cargo run --features anise --example solar_terms_year -- /path/to/de440s.bsp 2026 8
cargo run --features anise --example lunar_phases_year -- /path/to/de440s.bsp 2026 8
cargo run --features std --example spatial_catalog_motion
```

所有示例位于 [`examples/`](examples)。需要外部 BSP 或 EOP 的示例会通过命令行参数或错误消息明确说明输入要求。

## 开发与验证

```bash
cargo fmt --all --check
cargo check
cargo check --no-default-features
cargo test
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
```

依赖本地 DE440/EOP 数据的高成本契约测试默认标记为 `ignored`：

```bash
HYASTRO_DE440S=/path/to/de440s.bsp \
HYASTRO_EOP_FINALS=/path/to/finals2000A.all \
cargo test --all-features -- --ignored
```

项目要求 Rust `1.89` 或更高版本，使用 Rust 2024 edition，并在 crate 根启用 `#![forbid(unsafe_code)]` 与 `#![deny(missing_docs)]`。

## 项目文档

- [`CONTEXT.md`](CONTEXT.md)：领域词汇表
- [`docs/DOMAIN_MODEL.md`](docs/DOMAIN_MODEL.md)：领域模型与类型语义
- [`docs/PRD.md`](docs/PRD.md)：产品需求与验收边界
- [`docs/FEATURES.md`](docs/FEATURES.md)：原子功能目录与当前实现说明
- [`docs/CODE_STANDARDS.md`](docs/CODE_STANDARDS.md)：代码与数据工程规范
- [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md)：依赖和数据源决策

## License

本项目在 `Cargo.toml` 中声明为 [MIT](https://opensource.org/license/mit/) 许可。发布或再分发前，请同时核对所使用 JPL、IERS 和其他外部数据文件各自的来源与许可条款。
