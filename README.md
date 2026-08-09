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
| `ephem` | 强类型天体身份、相对位置速度、覆盖范围；可选 ANISE/JPL BSP 后端 |
| `astro` | 光行时、站心视差、太阳偏折、相对论光行差、真空/折射观测位置、太阳时、球形视盘与月球测光 |
| `event` | 升中落、球形盘面上缘/中心/下缘接触、晨昏蒙影、节气、月相、行星配置和多类极值事件 |
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
| `std` | 是 | 启用标准库、输入校验和 SOFA 适配 |
| `anise` | 否 | 启用 ANISE/JPL BSP 星历后端；同时启用 `std` 与 `hifitime` |
| `hifitime` | 否 | 启用 `hifitime` 时间适配 |
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
```

## JPL 星历与 IERS EOP

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
