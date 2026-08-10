# hyastro 依赖决策文档（DEPENDENCIES）

- 文档状态：依赖基线定稿（调研版）
- 调研基线：2026-08-05；crates.io 元数据查询日期同为 2026-08-05
- 配套文档：`docs/PRD.md`、`docs/FEATURES.md`、`docs/CODE_STANDARDS.md`、`docs/LIBRARY_RESEARCH.md`
- 实现进展：`Cargo.toml` 已锁定 `sofars = 0.6.1`，由默认 `std` feature 启用，用于 IAU 2006/2000A `GCRS → CIRS → TIRS → ITRS` 地球定向链、IAU 2006 日期平/真赤道与日期平黄道、Hipparcos ICRS↔IAU Galactic 转换、Fukushima (2006) 测地坐标与 ITRS 地心直角坐标双向转换、Fairhead–Bretagnon (1990) 完整地心 `TDB−TT` 解析模型，以及 `refco`/`atioq` 站心大气折射链；`hifitime` 保持可选模型 adapter。库内已实现 IERS EOP 20u24 C04 与 finals2000A 解析器，但不联网获取数据。

## 0. 方法与约定

### 0.1 状态分类（唯一状态枚举）

| 状态 | 含义 |
|---|---|
| **确定采用** | 进入主 crate 生产依赖；默认启用或经默认 feature 启用 |
| **可选 feature** | 以 Cargo feature 门控的生产依赖；默认关闭；启用即获得该能力 |
| **仅开发/验证** | 只出现在 `[dev-dependencies]` 或独立验证 crate，绝不进入生产依赖 |
| **仅工具链** | 不进入 Cargo 清单；作为 CI/发布工具使用 |
| **明确不采用** | 评估后拒绝；给出理由与替代 |

每个候选必须且只能落在上述一种状态。第 3 节为全部候选的完整条目。

### 0.2 最小直接依赖原则（决策基线）

1. **只有 hyastro 源码直接引用的 crate 才能出现在 `[dependencies]`**。ANISE 内部依赖（`bytes`/`memmap2`/`zerocopy`/`der`/`indexmap`/`crc32fast`/`tabled`/`const_format`/`snafu`/`log`/`ureq`/`url`/`regex`/`serde_dhall`/`serde-lexpr`/`hyperdual` 等）若 hyastro 源码不直接 `use`，一律不升格为直接依赖。
2. **workspace 统一版本、不重复抽象**：`nalgebra`、`hifitime`、`sofars` 等与 ANISE 同版本的 crate 在 `[workspace.dependencies]` 统一声明，但 hyastro 不因"ANISE 依赖了它"就把它当作自己的直接依赖。
3. **默认构建最小化**：默认 feature 只含 P0 最小内核（纯 Rust、离线、低依赖，满足 FEATURES.md F-FEAT-002）；网络、FFI、大型格式、并行均为可选。
4. **公开 API 类型隔离**：任何上游类型（ANISE `Almanac`/`Frame`/`CartesianState`、hifitime `Epoch`/`Duration`、sofars 裸 `f64` 数组）不得出现在 hyastro 公开接口；适配层私有封装，转换集中在接缝。
5. **禁止重复类型体系**：同一领域（时间、参考系、单位、状态、历表、错误）全库只允许一套规范类型（见 8.1）。

### 0.3 来源与标注

- 一手来源：crates.io API（版本/许可/`rust-version`/feature）、docs.rs、官方仓库源码/README/LICENSE/CI/CHANGELOG、标准组织文档（IAU SOFA 许可、IERS、IVOA、NAIF、CSPICE）。
- 标注：`[观察]` 直接读自源码/元数据；`[推断]` 基于观察的推理。`ref/<repo>/<path>:<行号>` 为本地精确路径。

## 1. 决策摘要总表

| 候选 | 状态 | 一句话结论 |
|---|---|---|
| hifitime | **确定采用** | 时间内核 4.3.x，MPL-2.0，无 unsafe，整数时间，no_std 支持 |
| sofars | **确定采用** | 天文算法内核 0.6.1，纯 Rust、0 unsafe，对照 SOFA C 2023-10-11 的 196 项官方数值测试 |
| thiserror | **确定采用** | 错误类型 2.x，零依赖，no_std 友好 |
| bitflags | **明确不采用** | 当前没有真实的公开位集合需求，领域状态使用枚举和独立类型 |
| libm | **确定采用** | `no_std` 数学内核的浮点函数；固定 `f64`，不引入数值泛型 |
| serde | **可选 feature `serde`** | 仅 hyastro 自有类型 derive 时直接依赖（P1） |
| rayon | **可选 feature `rayon`** | 批处理/并行查询（P1） |
| anise | **已采用可选 feature `anise`** | 固定 0.10.4、`default-features=false`，作为离线 SPK/DAF 历表后端；MPL-2.0 |
| tracing + tracing-log | **可选 feature `logging`** | 结构化日志 + `log` crate 桥接（观察 ANISE 内部日志） |
| csv | **可选 feature `catalog-csv`** | Gaia DR3 CSV 流式适配（P1） |
| winnow | **可选 feature `text-parsing`** | 自研文本格式解析器（P1/P2） |
| geographiclib-rs | **可选 feature `geodesy`** | 测地线/大地测量（P1） |
| vsop87 | **可选 feature `vsop87`** | 太阳/行星低精度历表后端（P1） |
| sha2 | **当前不采用** | 历表加载不要求或校验 SHA-256；内核路径、文件长度和冻结加载顺序保持显式 |
| flate2（miniz_oxide 后端） | **可选 feature `compression`** | gzip/deflate（P1，纯 Rust） |
| fitsrs =0.4.1 | **可选 feature `fits`** | CDS 纯 Rust FITS 多 HDU、图像和基础 BINTABLE 读取（P2） |
| votable（CDS） | **可选 feature `votable`** | VOTable 读/写（P2） |
| arrow + parquet | **可选 feature `parquet`** | 自转换 Parquet 与大星表列式访问（P2，重依赖） |
| cdshealpix | **可选 feature `healpix`** | HEALPix 索引（P2） |
| moc（CDS） | **可选 feature `moc`** | 多阶覆盖图（P2，依赖 healpix） |
| sgp4 | **可选 feature `sgp4`** | SGP4/TLE 传播（P2） |
| jiff | **可选 feature `timezone`** | IANA 时区互操作（P2） |
| zstd | **明确不采用（直接依赖）** | 仅在 `parquet` feature 需要时由 parquet 传递启用 |
| rand / rand_chacha / rand_distr | **仅开发/验证** | 蒙特卡洛验证、确定性采样测试 |
| approx | **仅开发/验证** | 浮点容差断言 |
| proptest | **仅开发/验证** | 属性测试 |
| trybuild | **仅开发/验证** | 编译期错误诊断测试 |
| rstest | **仅开发/验证** | 参数化测试（取舍见 4.3） |
| criterion | **仅开发/验证** | 基准（P1 起） |
| iai-callgrind | **仅开发/验证** | 指令级基准与分配计数 |
| arbitrary / libfuzzer-sys | **仅开发/验证** | fuzz 目标（ANISE 同款 25 目标模式） |
| tempfile | **仅开发/验证** | 临时数据文件 |
| rust-spice（CSPICE） | **仅开发/验证** | SPICE 差分 oracle（F-TEST-005） |
| rsofa | **仅开发/验证** | SOFA C 逐位对照 oracle（F-TEST-003/007） |
| novas | **仅开发/验证** | NOVAS C3.1 独立交叉验证（F-TEST-007；许可先澄清） |
| cargo-deny / cargo-audit / cargo-semver-checks / cargo-msrv / Miri / Kani / rustfmt / Clippy / cargo-fuzz | **仅工具链** | CI/发布/审计工具 |
| nalgebra | **明确不采用（直接依赖）** | 仅 ANISE 传递；MSRV 1.89、无天文语义、公开面禁止 |
| uom | **明确不采用** | 泛型维度系统与天文角语义不匹配 |
| num-traits / memmap2 / zerocopy / bytes / der / crc32fast / indexmap / tabled / const_format / snafu / log | **明确不采用（直接依赖）** | 仅为上游传递依赖或当前无直接用途，不升格（0.2 原则 1） |
| erfa-sys / erfa（crate） | **明确不采用** | 停更、系统库依赖、`links="erfa"` 独占、子模块未检出 |
| nyx-space | **明确不采用** | AGPL + `premium` 双许可、领域错位、依赖树重 |
| rust-astro（astro 2.0.0） | **明确不采用** | 2018 年停更、edition 2015；仅作移植蓝本（MIT） |
| argmin / roots / brentroot | **明确不采用** | 求根/极值自研（PRD MATH-NUM-002/003） |
| bzip2 | **明确不采用** | FFI + 低频用途 |
| fitsio | **明确不采用** | 生产基线不引入 cfitsio FFI；完整 FITS 写入需求须另立决策 |
| celestial（gaker） | **明确不采用** | 第二套领域模型（时间/帧/坐标），与 hyastro 强类型冲突 |
| celestial-eop-data | **明确不采用** | 丢失观测/预报标志，并把缺失预测字段写成零；hyastro 保留空列并执行显式来源接纳策略：UT1-only 查询只要求真实 `UT1−UTC`，完整姿态查询仍拒绝缺失字段 |
| uom 之外的其它单位 crate（dimensional 等） | **明确不采用** | 同 uom 理由 |
| polars | **明确不采用（生产）** | 仅 ANISE dev-deps 使用；hyastro 用 arrow/parquet 直接接口 |


## 2. ANISE 深入调研

### 2.1 身份与版本

- 仓库：`github.com/nyx-space/anise`；调研基线 = 本地 `ref/anise` HEAD `b5c4fba6c4bdcd78fa8382b376f7efee9a01b82c`（2026-08-01 提交，`git log -1`），相对 `v0.10.4` tag 领先 42 个提交（`git describe` 输出 `0.10.4-42-gb5c4fba`）。
- workspace 版本 0.10.4、edition 2024、`license = "MPL-2.0"`（`ref/anise/Cargo.toml`）；成员：`anise`（核心）、`anise-cli`、`anise-gui`、`anise-py`、`anise-cpp`、`anise/fuzz`。
- crates.io 最新稳定版 = **0.10.4**（2026-08-05 查询 `https://crates.io/api/v1/crates/anise`，`max_stable_version=0.10.4`）；发布包（`anise-0.10.4.crate`，约 370 KB）的归一化 `Cargo.toml` 与本地 workspace 依赖声明一致（含 `sofars = "0.6.1"` 硬依赖、`hifitime = "4.3.0"`、`nalgebra = "=0.35"`、`der 0.7.8`、`zerocopy 0.8.0`、`bytes 1.6.0`、`indexmap 2.11.4`、`memmap2 0.9.4`、`crc32fast 1.4.2`、`log 0.4`、`snafu 0.9.0(backtrace)`、`tabled =0.21`、`const_format 0.2`、`serde 1`）。
- 作者/维护：Christopher Rabotin（Nyx Space）；文档声明 NASA TRL 9（Firefly Blue Ghost 任务运营使用，`ref/anise/README.md:6-8`）。[观察]

### 2.2 许可：MPL-2.0

- workspace `license = "MPL-2.0"`（`ref/anise/Cargo.toml`），仓库根 LICENSE 为 MPL-2.0 全文（16.3 KB，与 MPL 2.0 官方文本一致 [观察]）。
- 含义：以**文件级 copyleft** 约束——修改 ANISE 源码的文件必须继续以 MPL-2.0 提供该文件源码；作为库链接/调用不传染 hyastro。因此 hyastro 采用 ANISE 时：不得修改 ANISE 源码后闭源再分发；不修改、只调用，则无开源义务（可商用闭源，与 hifitime 同款许可模式）。
- 与 hyastro 其它依赖的兼容性：MPL-2.0 与 MIT/Apache-2.0 代码同库链接无冲突；分发时需保留 ANISE 版权声明与许可文件副本（CODE_STANDARDS 15 节要求）。

### 2.3 MSRV / edition / 工具链

- ANISE 0.10.4 manifest 无 `rust-version` 字段，edition 2024 隐含 ≥1.85；其传递依赖 `nalgebra =0.35` 的 crates.io 元数据声明 `rust-version=1.89.0`。[观察]
- hyastro 采用 ANISE 后已把 manifest 的 `rust-version` 明确提升为 **1.89**，使声明 MSRV 与 `--all-features` 的有效依赖下限一致。保留 1.86 会迫使历表后端回退到自研 SPK 解析，重复 ANISE 已验证的实现。[决策]
- ANISE CI 使用 stable，无独立 MSRV 固定测试；hyastro 必须自行守住声明的 1.89 基线。
- 文档元数据小缺陷：`anise/Cargo.toml` 的 `[package.metadata.docs.rs]` 将 `rustdoc-args` 误写为 `rustdoc-ars`（发布包中同样存在），docs.rs 的 `--cfg docrs` 可能未生效。[观察]

### 2.4 模块与公开能力

入口 `anise/src/lib.rs:15-33`（`pub mod`）：`almanac`、`astro`、`constants`、`ephemerides`、`errors`、`frames`、`math`、`naif`、`orientations`、`structure`；`analysis` 由 `#[cfg(feature = "analysis")]` 门控；`time` 模块重导出 `hifitime::*`；`prelude` 重导出 Almanac/MetaAlmanac(metaload)/SPK/BPC/Frame/Orbit/Aberration/单位/NAIFSummaryRecord。

| 模块 | 公开能力 [观察] |
|---|---|
| `almanac` | `Almanac` 上下文（SPK/BPC/Planetary/Spacecraft/EulerParameter/Location/Instrument 数据集的 IndexMap 容器）；`transform`/`transform_to`/`state_of`/`spk_ezr`（SPICE `spkezr` 别名）、`translate`/`rotate`、`azimuth_elevation_range_sez`（地面站 AER）、`occultation`/`solar_eclipsing`/`line_of_sight_obstructed`、`beta_angle_deg`/`ltan`/`ltdn`、`frame_info` |
| `astro` | `Orbit`（Keplerian/geodetic/mean elements/equinoctial，`orbit.rs` 57.6 KB）、`Aberration`（9 种 SPICE 兼容模式）、`occultation`、`aberration::stellar_aberration` |
| `constants` | 帧/天体/定向常量与 NAIF ID 映射（`EARTH_ITRF93`、`EME2000`、`IAU_VENUS_FRAME` 等，30.7 KB） |
| `ephemerides` | `Ephemeris`（OEM/OPM/STK 解析与写入，`opm.rs` 29.6 KB）、SPK 写入（`to_spice_bsp`，Type 9/12/13）、`translations`/`paths`（光行时求值） |
| `errors` | snafu 错误枚举（`AlmanacError`/`EphemerisError`/`OrientationError` 等，`errors.rs` 8.3 KB） |
| `frames` | `Frame`（Copy 结构：ephemeris_id + orientation_id + force_inertial + mu + shape + frozen_epoch）、`FrameUid`、`DynamicFrame`（18.4 KB，运行时建帧） |
| `math` | `CartesianState`（26.3 KB）、`Vector3`/`Matrix3`/`DCM`/`Quaternion`/`MRP`/`EulerParameter`（`math/rotation/`）、Chebyshev/Hermite/Lagrange 插值、单位（`units.rs`） |
| `naif` | DAF 通用读写（`daf/daf.rs` 42.3 KB，含 `mut_daf` 写路径、CRC32 校验、大/小端处理）、SPK/BPC 摘要、KPL 文本解析（`kpl/parser.rs` 30.6 KB，FK/TPC 转换 `convert_tpc`）、PCK |
| `orientations` | 旋转路径求解、动态旋转（含 sofars 岁差/章动模型）、BPC 求值 |
| `structure` | 数据集类型（Planetary/Spacecraft/EulerParameter/Location/Instrument）、`Location`（经纬高 + 帧 + 地形掩膜）、LookupTable、Metadata（ANISE 自定义 ASN.1 DER 头） |
| `analysis`（feature） | 事件搜索（`adaptive_step_scanner`/`brent_solver`）、S-表达式 DSL（`expr.rs`）、报告、DCM 表达式 |

### 2.5 格式支持矩阵（SPK / PCK / FK / BPC / DAF）

**读取分派实况**（`ref/anise/anise/src/ephemerides/translate_to_parent.rs:59-120` 的 `match summary.data_type()`，最终证据）：实现求值的类型为 **Type 1（Modified Difference）、Type 2（Chebyshev Triplet）、Type 3（Chebyshev Sextuplet）、Type 8（Lagrange Equal Step）、Type 9（Lagrange Unequal Step）、Type 12（Hermite Equal Step）、Type 13（Hermite Unequal Step）**；其余一律返回 `DAFError::UnsupportedDatatype`。`DataType` 枚举可识别 1-21 全部类型号（`data_types.rs:29-44`），但 5/10/14/15/17/18/19/20/21 无实现。

| SPK 类型 | 状态 | 依据 |
|---|---|---|
| 1 修正差分 / 2 Chebyshev 三元组 / 3 Chebyshev 六元组 | ✅ 稳定 | translate_to_parent.rs:59-80；CI 有 type01/type02/type03 差分验证（tests/ephemerides/validation/） |
| 9 Lagrange 不等步 | ✅ 稳定 | translate_to_parent.rs:95-101；CI type09 验证 |
| 13 Hermite 不等步 | ✅ 稳定 | translate_to_parent.rs:113-119；CI type13 验证 |
| 8 Lagrange 等步 / 12 Hermite 等步 | 🧪 已实现、无公开内核样本验证 | translate_to_parent.rs:86-94、104-112；README 表标注 🧪"supported but no public SPK of that type could be found to validate" |
| 5 离散状态 / 10 TLE / 14-21 各型 | ❌ 不支持 | translate_to_parent.rs `dtype => UnsupportedDatatype`；README 表 ❌ |
| 写入（to_spice_bsp） | 仅 Type 9/12/13 | `ephemerides/ephemeris/spk.rs:45-56` |

**其它核**：BPC（二进制行星常数/高保真旋转）✅ 原样支持（`naif/mod.rs` `pub type BPC = DAF<BPCSummaryRecord>`）；PCK 文本（TPC）与 FK（帧核）、TK/LSK/GM 需先经 ANISE 转换（`convert_tpc`/`kpl/parser.rs`，产出 `.pca`/`.epa`/`.lka` 等 ANISE 自有格式）；CK/SCLK/DSK/IK/EK ❌（README 支持表"Yet to be supported"）。DAF 本体支持读、写（`mut_daf`）与 CRC32 校验；字节序按文件头 `LTL-IEEE`/`BIG-IEEE` 处理（`naif/mod.rs` `Endian`）。

### 2.6 时间 / 参考系 / 光行时 / 地面站

- **时间**：时间语义完全由 hifitime 提供（`time` 模块重导出；内部历元为 TDB 秒 + `Epoch`）。覆盖 TT/TAI/ET/TDB/UTC/GPS 等 13 时标、闰秒表、UT1 通道（hifitime `ut1` feature）。SPICE 差分中 ET↔UTC 与 SPICE 零差（hifitime README 声明，`ref/hifitime/README.md:249`）。
- **参考系**：帧 = NAIF ID 组合（ephemeris_id + orientation_id），`Frame` 带 `force_inertial`/`frozen_epoch`/`mu`/`shape` 元数据；变换 API `transform`/`rotate` 每次校验帧合法性（frame safety）；动态帧（`frames/dynamic.rs`）支持用户定义；**注意**：ANISE 帧体系 = SPICE/NAIF 体系（J2000/ITRF93/IAU 行星帧等），**不提供** IAU 2006/2000A 完整地球定向链与 GCRS/CIRS/TIRS/ITRS 参考系类型——hyastro 的 EOP/地球定向链仍须以 sofars 为内核自建，ANISE 仅作 SPICE 系帧后端（与 LIBRARY_RESEARCH 第 4 节矩阵一致）。
- **光行时**：`Aberration` 9 模式（NONE/LT/LT+S/CN/CN+S/XLT/XLT+S/XCN/XCN+S；接收/发射、迭代/非迭代、恒星光行差），`aberration.rs:53-95`；迭代策略：非收敛 1 次、收敛 3 次迭代 + 光行时变率修正（`translations.rs:142-179`，`num_it = if ab_corr.converged { 3 } else { 1 }`）；差分验证：DE440s 上 101,000 对查询，非收敛光行时 99 分位误差 < 5 m、75 分位 < 1 m、中位数 < 2 mm（`aberration.rs` 文档注释）。
- **地面站**：`Location`（纬度/经度/高度/帧/地形掩膜，`structure/location.rs:28-47`）+ `azimuth_elevation_range_sez`（`almanac/aer.rs:81`）与按 ID/名字查询；`Orbit::try_latlongalt` 构造站址状态（README 示例）。

### 2.7 线程安全与 unsafe

- **线程安全**：`Almanac` 为 `Clone + Default` 的纯数据容器（`almanac/mod.rs:66-82`：`IndexMap<String, SPK>`、`IndexMap<String, BPC>`、数据集 IndexMap），全部字段为 owned 数据；全库无 `Mutex`/`RefCell`/`static mut`/`lazy_static`/`OnceLock`（grep 为 0 命中）[观察]；`Send + Sync` 由字段类型推出 [推断]。查询 API 均取 `&self`。并行能力仅在 `analysis` 与 `python` feature 内经 rayon 提供（`almanac/python.rs` 三处 `par_iter`、`analysis/mod.rs:19`）。
- **unsafe 范围**：`anise/src` 全目录仅 **1 行含 `unsafe`**（`lib.rs:92`，`file_mmap!` 宏内的 `memmap2::MmapOptions::map` 调用）；tests 中仅 `tests/orientations/validation.rs` 因调用 CSPICE FFI 含 unsafe。生产库 unsafe 面积 ≈ 0。

### 2.8 features 与 default-features=false 的可用能力

`anise/Cargo.toml:71-77`：

```toml
default = ["metaload", "analysis"]
python = ["pyo3", "pyo3-log", "numpy", "ndarray", "rayon", "hifitime/python"]
metaload = ["url", "ureq", "platform-dirs", "regex", "serde_dhall"]
embed_ephem = ["rust-embed", "ureq"]
analysis = ["rayon", "serde-lexpr", "csv", "hyperdual"]
validation = []
```

- **`metaload`（默认开）**：`MetaAlmanac`（Dhall 配置、自动下载、CRC32 校验、AppData 缓存、进程锁）；引入 ureq+rustls（HTTP 客户端）、url、platform-dirs、regex、serde_dhall。**联网能力**。
- **`analysis`（默认开）**：事件/表达式引擎；引入 rayon、serde-lexpr、csv、hyperdual（自动微分）。
- **`python`**：pyo3/numpy/ndarray 绑定（hyastro 不启用）。
- **`embed_ephem`**：build.rs 在**构建期联网下载** pck11.pca 与 de440s.bsp 并内嵌（`anise/build.rs`，200 MB 上限）。
- **`validation`**：空 feature，仅标记差分测试（`--features validation` 跑 `validate_jplde_*` 等）。
- **`default-features=false` 的可用能力**：`Almanac` 全部核心能力（SPK/BPC/PCK/FK/TPC 加载、translate/rotate/transform/AER/eclipse、`structure` 数据集、`ephemerides` OEM/OPM/STK、帧、math、DAF 读写）+ 硬依赖（hifitime/sofars/nalgebra/der/zerocopy/bytes/…）——**不联网、无 rayon、无 hyperdual**。CI 显式跑 `cargo test --no-run --no-default-features`（rust.yml），证明该配置可构建。

### 2.9 网络与缓存

- 下载端点：JPL `https://naif.jpl.nasa.gov/pub/naif/generic_kernels/...` 与 Nyx `http://public-data.nyxspace.com/anise/...`（**注意 Nyx 端为明文 HTTP**，`metaalmanac.rs` `Default` 实现中 `Url::parse("http://public-data.nyxspace.com/anise/")`）。[观察]
- `MetaAlmanac::default()` 固定拉取 5 个文件：de440s.bsp（CRC32 `0x7286750a`）、pck11.pca（`0x1edb3eac`）、moon_fk_de440.epa（`0xc6c252fa`）、moon_pa_de440_200625.bpc（`0xcde5ca7d`）、earth_latest_high_prec.bpc（无 CRC，**每日更新**，文档注明两次查询结果可能不同——可复现性风险）。
- 缓存：本地已存在文件按 CRC32 匹配则复用；下载落到平台 AppData（`platform-dirs`）；下载期间写锁文件防并发（`autodelete` 10 秒死锁回收）。
- **hyastro 决策**：核心离线（PRD 4.2 禁止隐式联网）→ 默认不启用 `metaload`；hyastro 自有数据获取工具（HTTPS + 校验和 + 版本固定）替代自动下载。

### 2.10 依赖体积

- 发布包：`anise-0.10.4.crate` ≈ 370 KB；源码 LOC：`anise/src` 42,573 行（`wc -l` 实测）。
- 硬依赖（default-features=false 仍存在）：hifitime 4.3、sofars 0.6.1、nalgebra =0.35（serde-serialize+std，关默认特性）、der 0.7.8（derive/alloc/real）、zerocopy 0.8（derive）、bytes 1.6、indexmap 2.11、memmap2 0.9.4、crc32fast 1.4.2、log 0.4、snafu 0.9（backtrace）、tabled =0.21、const_format 0.2、serde 1 + serde_derive 1。
- 传递抬升：nalgebra 0.35 → MSRV 1.89；zerocopy/bytes 等无显著抬升（rust-version 1.56/1.57）。
- 对比：作为历表/帧后端，比直接引入 CSPICE/rust-spice（FFI、全局状态、C 工具链）轻且纯 Rust。

### 2.11 测试与 SPICE 差分

- 差分基础设施：dev-dependency `rust-spice = "0.7.6"`（CSPICE FFI）+ `validation` feature；`tests/orientations/validation.rs`（770 行）与 `tests/ephemerides/validation/{type01,type02,type03,type09,type13,validate,compare}.rs` 逐段对照 SPICE。
- 数量（`anise/README.md:172`、根 `README.md:128`）：DE440.bsp 上 **101,000 次查询**；PCK08 每个帧 **7,305 次查询**（20 年逐日）；地球高保真 BPC 数千次旋转。
- 容差（`tests/orientations/validation.rs:33-40`）：旋转角误差 ≤ 2″（`MAX_ERR_DEG=7.2e-6`）、DCM 元素 ≤ 2e-9；**IAU 月球帧例外容差 1e-3°**（SPICE 浮点世纪计算与 hifitime 整数时间的已知差异，最高约 1 毫度，README 注明）；位置 2e-5 km、速度 5e-7 km/s。
- CI 步骤（rust.yml）：`validate_gh_283_multi_barycenter_and_los`、`validate_jplde`（Type 2/3）、`validate_hermite_type13_`、`validate_lagrange_type9_*`、`validate_modified_diff_type01`、`validate_iau_rotation_to_parent`、`validate_bpc_*`、`de440s_translation_verif_venus2emb`——全部 `--include-ignored --test-threads 1`（差分测试要求串行）。
- 其它质量设施：25 个 libfuzzer fuzz 目标（`anise/fuzz/`，覆盖 BPC/SPK 解析、KPL/FK/TPC 解析、旋转/四元数/MRP 代数、数据集反序列化）；criterion 0.8 + iai-callgrind 0.16 基准（6 个 bench）；代码覆盖率经 cargo-llvm-cov 上报。

### 2.12 接入方式（hyastro 侧设计）

1. **feature 门控**：已配置 `[dependencies] anise = { version = "=0.10.4", default-features = false, optional = true }`；feature `anise` 同时启用 `std` 与现有 `hifitime` adapter。不启用 `metaload`、`analysis`、`embed_ephem`、`python` 或 `validation`。
2. **类型隔离**：公开接口只出现 hyastro 的 `Ephemeris`、`EphemerisQuery<Bcrs, S>`、`RelativeState<Bcrs, S>`、`Coverage`、`CelestialBody` 和 `KernelManifest`。ANISE `Almanac`、`Frame` 与 `CartesianState` 均留在私有实现；相对状态以显式目标/中心和自由向量表示，不误用固定 SSB 原点的 `Point3<Bcrs>`。
3. **版本统一**：hyastro 直接依赖固定 `anise =0.10.4`、兼容 `hifitime =4.3` 与 `sofars =0.6.1`；`nalgebra`、`der`、`zerocopy` 等仅保持 ANISE 传递依赖，不升格为直接依赖。
4. **错误转换**：ANISE 的错误在适配层映射为稳定的 `UnknownTarget`、`UnknownCenter`、`Coverage`、`UnsupportedFrame`、`UnsupportedSegment`、`CenterCycle`、`CorruptKernel` 或 `Backend`，公开错误不含上游类型。
5. **数据策略**：调用者显式提供本地内核路径；`KernelManifest` 冻结加载顺序并记录文件长度，后列内核维持较高重叠优先级。运行时不联网、不读取 `latest`、不要求或校验 SHA-256。ANISE 仍负责 DAF 结构解析与段边界检查。

### 2.13 风险清单

| # | 风险 | 等级 | 缓解 |
|---|---|---|---|
| 1 | 传递 MSRV 1.89（nalgebra 0.35） | 中 | 全库 MSRV 基线记入；文档化；若收紧需替代帧后端 |
| 2 | `metaload`/`embed_ephem` 联网、Nyx 端为明文 HTTP | 中 | 默认关闭；hyastro 数据管线 HTTPS + CRC32/sha256 |
| 3 | `earth_latest_high_prec.bpc` 每日更新致可复现性漂移 | 中 | 固定版本快照；结果元数据记录数据日期 |
| 4 | SPICE/ANISE 旋转差分上限 ~1 毫度（IAU 帧，整数时间 vs 浮点世纪） | 低 | 差分测试容差对齐（validation.rs 已处理）；hyastro 以 hifitime 语义为准 |
| 5 | MPL-2.0 文件级 copyleft：修改 ANISE 源码须回馈 | 低 | 不改源码；如需补丁走上游 PR；分发保留许可 |
| 6 | 0.10.x 快速迭代（0.10.4+42 commits），API 变动风险 | 中 | semver 范围 + cargo-semver-checks；适配层薄 |
| 7 | 帧/时间语义为 SPICE/NAIF 体系，非 IAU 2006 完整链 | 中 | hyastro 地球定向链用 sofars 自建；ANISE 仅作历表/SPICE 帧 |
| 8 | CK/SCLK/DSK/IK/EK、SPK 5/10/14-21 不支持 | 低 | 需求核对：hyastro P0-P2 只用 1/2/3/9/13（DE 系）+ BPC/PCK/FK |
| 9 | 唯一 unsafe（`file_mmap`）与 zerocopy 零拷贝解析 | 极低 | 审计该宏用途；fuzz 已覆盖解析路径 |
| 10 | `Epoch::now()`（系统时钟）仅在无参数据集命名路径使用 [观察] | 极低 | 不依赖该路径 |

### 2.14 结论

ANISE 0.10.4 满足"生产级 SPICE/DAF/SPK/PCK/参考系后端"定位：纯 Rust 核心（unsafe≈1 行）、无全局状态线程安全、10 万级 SPICE 差分验证、MPL-2.0 商用友好、`default-features=false` 下离线可用。**决策：确定为"可选 feature `anise`"**（默认关闭；启用时 `default-features=false`），与 hifitime + sofars 内核并存：hifitime/sofars 提供时间与 IAU 天文算法，ANISE 提供 SPICE 系历表/内核/帧后端，两者由 hyastro 自有类型层统一。不引入 ANISE 的 metaload/analysis/embed_ephem/python。


## 3. 依赖逐项评估与决策

条目字段：用途 / 建议版本约束 / features / 默认 / no_std / unsafe·FFI / 许可 / MSRV / 平台 / 为何不自己实现 / 替代项及理由。版本与许可数据均来自 crates.io API 2026-08-05 查询（附录 B），另有注明者除外。

### 3.1 基础库

#### 3.1.1 thiserror —— **确定采用**（P0）

- 用途：统一错误类型派生（hyastro 全部公开错误枚举，CODE_STANDARDS 16 节"错误枚举 `#[non_exhaustive]`"配合使用）。
- 版本约束：`thiserror = "2"`（crates.io 最新 2.0.19；1.x 兼容但 2.0 起默认 edition 2021 且 MSRV 1.61+，2.0.19 声明 `rust-version=1.71`）。
- features：无；默认即用。默认：启用。
- no_std：支持（`thiserror-core` 拆分后 2.x 保持 `#![no_std]` 能力）[观察：crate 文档声明]。
- unsafe/FFI：无。许可：MIT OR Apache-2.0。MSRV：1.71。平台：全平台。
- 为何不自己实现：错误派生（Display/From/backtrace/context）是纯样板，自研无收益；thiserror 零依赖、生态标准。
- 替代项：snafu（ANISE/hifitime 在用，但 context 机制侵入式、生成额外结构体，不适合作为 hyastro 对外错误面）；anyhow（仅应用层，库不用）；`derive_more`（功能更泛但面更大）。理由：thiserror 是 Rust 库错误的标准解，符合产品决策"错误用 thiserror 类方案"。

#### 3.1.2 serde / serde_derive —— **可选 feature `serde`**（P1）

- 用途：hyastro 自有领域类型的序列化（PRD P1"Serde 互操作"；F-FEAT-003 要求 `serde` 独立 feature）。注意：即使不直接依赖，hifitime(std) 与 anise 也会传递引入 serde；直接依赖仅为派生自己的类型。
- 版本约束：`serde = { version = "1", features = ["derive"], optional = true }`（crates.io 最新 1.0.229，`rust-version=1.56`）。
- 默认：关闭（`serde` feature 开启时启用）。
- no_std：`serde` 本体支持；`derive` feature 需 proc-macro（构建期 std）。unsafe/FFI：无。
- 许可：MIT OR Apache-2.0。MSRV：1.56。平台：全平台。
- 为何不自己实现：序列化协议互操作（JSON/bincode/postcard 等）必须用生态标准 derive。
- 替代项：`serde` 无可替代（生态事实标准）。Rust 原生 `#[derive]` 只覆盖 Debug/Clone 等，不解决数据交换。

#### 3.1.3 bitflags —— **明确不采用**

- 当前领域模型没有真实的多选位集合需求。
- 模型选择使用封闭枚举，位置修正阶段使用不同结果类型。
- 出现无法由枚举或类型状态清晰表达的真实位集合后再重新评估。

#### 3.1.4 tracing / tracing-log —— **可选 feature `logging`**（P1）

- 用途：hyastro 结构化日志（span/事件/字段）；`tracing-log` 提供 `log` crate → tracing 的桥接（`LogTracer`），从而捕获 ANISE（`log` crate）与 hifitime 的内部日志，统一输出。
- 版本约束：`tracing = "0.1"`（最新 0.1.44，`rust-version=1.65`）、`tracing-log = "0.2"`（最新 0.2.0）。features：`tracing` 不开 `log`（用 tracing-log 单向桥接即可）；订阅端（tracing-subscriber）仅作 dev 依赖或示例。
- 默认：关闭（`logging` feature 开启时启用）。no_std：`tracing` 支持 `no_std`（关 `std`）；此处仅 std 路径。unsafe/FFI：无。
- 许可：tracing MIT；tracing-log MIT OR Apache-2.0。MSRV：1.65。平台：全平台。
- 为何不自己实现：日志分层/过滤/格式化是通用基础设施。
- 替代项：直接依赖 `log` crate（与 ANISE 同款，但无结构化 span 能力）；`slog`（生态小）。理由：tracing 是当前事实标准，`tracing-log` 桥接让 ANISE 的 `log!` 输出纳入同一管道；核心计算路径零日志（F-PERF-001 零分配不受影响，tracing 默认 no-op）。

### 3.2 数学 / 单位 / 时间

#### 3.2.1 nalgebra —— **明确不采用（直接依赖）**，仅 ANISE 传递

- crates.io 最新 0.35.0；Apache-2.0；`rust-version=1.89.0`（edition 2024）。
- 理由：(1) 泛型矩阵代数类型不携带天文语义（参考系/原点/单位/角语义），公开面使用即破坏 PRD 4.1 强类型；(2) `rust-version=1.89` 过高，会被动抬升 hyastro MSRV；(3) hyastro 的数学面是固定 `3×3`/`6×6`/四元数 + 强类型包装，自研实现（约数百行，纯 `f64`，可 no_std）可控且零依赖；(4) ANISE 已在 workspace 引入 =0.35（关默认、`serde-serialize`+`std`），任何需要 nalgebra 表达式的内部算法也可在 ANISE 适配器内进行，不扩散。
- 替代：hyastro 自有 `Vec3`/`Mat3`/`Quaternion`/`StateTransform`（MATH-VEC/ROT 全部需求）。

#### 3.2.2 uom —— **明确不采用**

- crates.io 最新 0.38.0；Apache-2.0 OR MIT；`rust-version=1.68`。
- 理由：(1) 泛型维度系统把"角度"当作单一 SI 维度，无法区分赤经/纬度/时角/方位/高度/相位角——正是 PRD MATH-ANG-002 禁止混用的语义；(2) 泛型 `Quantity<D,U,V>` 显著增加编译时间与泛型传播，与 PRD 6.1.2 MATH-QTY-004"核心算法以 f64 为规范、不扩散泛型"冲突；(3) 常数版本化（MATH-CON-002/004）需要 hyastro 自有常数组而非 uom 的静态换算。
- 替代：hyastro 自有量类型（长度/速度/角/角速度/频率/质量/压力/温度/湿度/波长），每量一个语义类型或受约束包装，换算表版本化。

#### 3.2.3 libm / num-traits —— **libm 确定采用**；**num-traits 不直接采用**

- 用途：`libm` 为 hyastro 固定 `f64` 的数学内核提供 `sin`、`cos`、`atan2`、`sqrt` 等函数，并保证 `std` 与 `no_std` 路径使用同一实现。
- 版本约束：`libm = "0.2"`（最新 0.2.16，`rust-version=1.63`）。`num-traits` 0.2 仅由 hifitime、SGP4 等上游按需传递，不进入 hyastro 直接依赖。
- no_std：libm 支持；unsafe/FFI：无；许可：MIT；平台：全平台。
- 决策理由：PRD MATH-QTY-004 规定核心算法使用固定 `f64`，公开接口不扩散浮点泛型，因此 hyastro 不直接使用 `Float`、`FromPrimitive`、`Zero` 或 `One`。为一个未使用的泛型接缝直接依赖 num-traits 会违反最小直接依赖原则。
- 替代项：`std` 的 `f64` 方法不覆盖 `no_std` 目标；`micromath` 精度不足。libm 是此处最小且可审计的直接依赖。

#### 3.2.4 hifitime —— **确定采用**（P0）

- 用途：为 hyastro 强类型时间值提供 TAI/TT/TDB/TCG/TCB/GPS 数值转换、上游 `Epoch` 互操作和交叉验证。默认 `std` 的天体测量/解析星历路径需要 TCB/TDB 转换，因此随 `std` 启用；裁剪构建也可单独选择 `hifitime` feature。UTC 标签解析仍由 hyastro 的版本化 `LeapSeconds` 决定，不把 Hifitime 的内嵌表作为公开语义。
- 版本约束：`hifitime = { version = "4.3", default-features = false, optional = true }`。约束 `"4.3"` 允许 4.3.x 内跟进，ANISE workspace 的 `"4.3.0"` 与之兼容；依赖仍以 Cargo optional 形式存在，由 `std`、`hifitime` 或 `anise` 功能图显式激活。
- features：hyastro 的默认 `std` feature 启用依赖及 `hifitime/std`；独立 `hifitime` feature 启用依赖本身。**不开** `ut1`（ureq 联网下载 EOP）、`lts`（联网比对 IANA 闰秒）、`python`；核心 `LeapSeconds` 不依赖这些 feature。
- no_std：Hifitime 支持 `no_std`（`ref/hifitime/src/lib.rs:3`）；hyastro 的 `--no-default-features` 核心不拉取该依赖，显式 `hifitime` 的裁剪组合可使用其无标准库子集。
- unsafe/FFI：0 处 unsafe；Kani 形式化验证工作流（`.github/workflows/formal_verification.yml`）。
- 许可：MPL-2.0（`ref/hifitime/LICENSE.txt`）。MSRV：manifest 无 `rust-version`；CI 以 1.85 为 MSRV；std 路径经 snafu `rust_1_81` 实需 1.81+。平台：全平台（含 wasm，`web-time` 提供时钟）。
- 为何保留：相对论时间尺度转换正确性极难自证；Hifitime 有 Kani 验证、完整测试以及与 SPICE 的 ET/UTC 对照，可作为算法 adapter 和独立校验源。闰秒版本、覆盖和过期属于 hyastro 领域不变量，因此由 hyastro 自己保存。
- 替代项：`chrono`（无 TAI/TT/TDB/UTC 闰秒语义）、`time`（民用）、`jiff`（民用 + tz，无天文时标）、`julian`（过窄）。Hifitime 仍是覆盖多种天文时标的活跃纯 Rust 候选，但不再拥有 hyastro 的 UTC 数据策略。
- 注意：Hifitime、SOFA `dat` 和 IANA 表对 1972 年前 UTC 的语义不同。hyastro 当前 `LeapSeconds::builtin` 明确从 1972-01-01 开始，超出覆盖返回错误；1960–1971 分段漂移以后作为独立 UTC 历史模型实现，不伪装成 `LeapSecond`。

#### 3.2.5 sofars —— **确定采用**（P0）

- 用途：IAU SOFA 纯 Rust 数值内核：时间尺度（21 函数）、地球定向（ERA/GMST/GAST/岁差/章动/极移，`erst`+`pnp` 全族）、IAU 2006 `ecm06/eqec06/eceq06` 日期平/真黄道、`astro::ab` 相对论周年光行差、`pmpx` 星表源站心方向、`starpv` / `pvstar` 六参数与质心状态互转、`starpm` 空间运动历元传播、Hipparcos `icrs2g/g2icrs` 规范银道转换、FK4/FK5、历表（epv00/moon98/plan94）、投影和向量矩阵（`vm`）。ANISE 自身也硬依赖 sofars 0.6.1（`ref/anise/anise/Cargo.toml`，用于 `orientations/dynamic.rs` 的 IAU1976/2000/2006 岁差与 1980/2000A/2000B/2006A 章动）——hyastro 与 ANISE 使用同一数值实现，无重复内核。
- 版本约束：`sofars = "=0.6.1"`（crates.io 最新 0.6.1 = 本地 HEAD 版本；0.x 有破坏性 API 变更历史——0.5.0 将 pnp API 从可变引用参数改为返回值，`ref/sofars/CHANGELOG.md`——故用精确版本锁定，升级走显式流程）。
- features：无。默认：启用（P0）。
- no_std：**不支持**（edition 2024、无 no_std 特性）——hyastro no_std 内核不得依赖 sofars，相关算法在 `no_std` 内核中自研或经特征门控（`std` 路径用 sofars 数值核对）。
- unsafe/FFI：0 处 unsafe（纯 Rust 移植，`ref/sofars/src` grep 计数为 0）。许可：MIT + SOFA 许可条款（`ref/sofars/LICENSE` 附 SOFA 六条条款：派生命名不得含 `iau`/`sofa` 前缀——sofars 已合规；再分发需声明差异；出版物致谢）。
- MSRV：README 徽章 1.85+（edition 2024 隐含）。平台：全平台。
- 为何不自己实现：230/247 SOFA 函数 + 196 项对照 SOFA C 官方数值的测试（`ref/sofars/tests/`，容差体系 `tests/common/mod.rs`），自研即重造整个 SOFA 并失去权威对照。
- 当前默认 `std` 路径把 `epv00`、`moon98` 和 `plan94` 封装为 `SofaAnalyticEphemeris`：通过 hyastro 自有 `EphemerisProvider` 返回 BCRS 相对状态、按目标组合求交的连续覆盖、稳定错误和模型 provenance，不暴露 sofars 数组。该后端覆盖 SSB/太阳/地球/月球及水星至海王星系统质心，公开 SOFA 的逐系统 PLAN94 误差统计，用于快速近似与无 BSP 测试；它不是 ANISE/JPL 后端的静默回退。
- 替代项：rsofa（FFI 直绑，仅作 oracle）；erfa-sys/erfa（不完整/停更）；自建（成本高）。理由：纯 Rust、0 unsafe、官方数值测试、MPL-2.0 无关的宽松商用许可；唯一缺口是 17 个 `vm`/`ts` 工具函数（`cpv`/`p2s`/`rm2v`/`tf2d` 等，见 LIBRARY_RESEARCH 3.1），由 hyastro `math` 层补齐（约 100 行纯数学）。


### 3.3 天文算法、历表与格式

#### 3.3.1 anise —— **可选 feature `anise`**（见第 2 节深入调研）

- 决策要点复述：`anise = { version = "=0.10.4", default-features = false, optional = true }`；不启用 metaload/analysis/embed_ephem/python/validation；适配层私有持有 `Almanac`，公开具体类型为 `Ephemeris`，并实现与解析后端相同的 `EphemerisProvider`。`Astrometry` 和 `Events` 只通过该接缝查询观测者接收状态和目标发射状态；真实 DE440s 契约测试再以 ANISE `CN_S` 方向作差分验证。版本/许可/MSRV/风险详见 2.2-2.13。

#### 3.3.2 winnow —— **可选 feature `text-parsing`**（P1/P2）

- 用途：自研文本格式解析器（ISO8601 扩展、DMS/HMS 文本、自定义星表/配置文件、未来 KPL 扩展）的 parser combinator 基础。
- 版本约束：`winnow = "1"`（crates.io 最新 1.0.4，`rust-version=1.65`）。features：无。默认：关闭。
- no_std：支持（`alloc`）。unsafe/FFI：无。许可：MIT。平台：全平台。
- 为何不自己实现：手写递归下降解析器的错误恢复/组合是成熟领域；winnow 无 unsafe、编译快、文档好。
- 替代项：`nom`（维护模式、panic 风险点更多）、`pest`（DSL 运行时开销）、手写。理由：winnow 为 nom 的社区继任项目（维护者 epage），API 更安全；仅在确实需要自研文本格式时启用——SOFA 级解析需求（KPL/TPC/FK）已由 ANISE 覆盖，本 feature 面向 hyastro 自有格式。

#### 3.3.3 csv —— **可选 feature `catalog-csv`**（P1）

- 用途：Gaia DR3 `gaia_source` CSV 与官方 ECSV(gzip) 数据体的流式读取（CAT-DAT-005）、Hipparcos/Tycho CSV 适配器和用户自定义列映射。`csv` 支持有界内存逐行读取、serde derive 反序列化，并可用 `ReaderBuilder::comment(Some(b'#'))` 跳过 ECSV 的 YAML 注释头；若要校验 ECSV 中的单位和数据类型，仍须由 hyastro 的薄适配层解析该头。
- 版本约束：`csv = "1"`（最新 1.4.0，`rust-version=1.73`）。features：无。默认：关闭。
- no_std：不支持（std io；底层 `csv-core` 支持 no_std）。无 FFI；源码有 4 处非 FFI `unsafe`，均为已验证 UTF-8 的零拷贝快速路径。许可：Unlicense OR MIT。平台：全平台。
- 为何不自己实现：CSV 状态机（引号/转义/CRLF/BOM/非 UTF-8 字段）成熟且易错。
- 替代项：手写 `split(',')`（错误：引号内逗号、多行字段）；`csv-core`（更底层，若需极简）。Rust 生态当前没有完整 ECSV crate；Gaia ECSV 采用“YAML 注释头薄适配 + `csv` 数据体”的组合。

#### 3.3.4 vsop87 —— **可选 feature `vsop87`**（P1）

- 用途：太阳/行星低精度历表后端（PRD P1"VSOP87 后端、太阳/月球/行星表观位置"）。Razican/vsop87-rs 提供 VSOP87A-E 全套系数与求值（crates.io 最新 3.0.0，Apache-2.0，纯 Rust）。
- 版本约束：`vsop87 = "3"`（维护活跃度：仓库最后推送 2024-04 [观察 GitHub API]，中低活跃；风险可控因为 VSOP87 系数冻结）。
- 默认：关闭。no_std：支持（无 std 需求 [推断]）。unsafe/FFI：无。许可：Apache-2.0。MSRV：未声明（edition 2018 [推断]）。
- 为何不自己实现：VSOP87D 全行星级数系数体积大（约 10 万项级数），内嵌数据与管理成本高；crate 已封装。
- 替代项：从 rust-astro（MIT，2018）移植 VSOP87D 系数（LIBRARY_RESEARCH 蓝本方案）——工作量相当但失去独立维护；`jpl_ephemeris`（依赖 DE 文件）。理由：采用现成纯 Rust crate 更快；若其维护停摆，系数属公开数据可自行 fork 内嵌。精度定位：角秒级（相对 DE440 为低精度后备），用于太阳/行星视位置与事件初值，高精度路径仍走 ANISE SPK。

#### 3.3.5 sha2 —— **可选 feature `integrity`**（P1）

- 用途：校验用户提供的 SPK/PCK 和星表文件，拒绝损坏或与期望摘要不符的数据。
- 版本约束：`sha2 = "0.11"`（最新 0.11.0，edition 2024，`rust-version=1.85`；保守可 `"0.10"` 0.10.9）。默认：关闭。
- no_std：支持。unsafe/FFI：无（RustCrypto 纯 Rust；SIMD 加速 feature 不启用）。许可：MIT OR Apache-2.0。平台：全平台。
- 为何不自己实现：密码学哈希不得自研。
- 替代项：`blake3`（更快，MMM 依赖小）、crc32fast（ANISE 已用，非密码学）。理由：SHA-256 是数据校验生态默认；crc32 仅作 ANISE 文件快照校验（沿用 ANISE 语义），sha2 用于 hyastro 自有管线的强校验。

#### 3.3.6 flate2 —— **可选 feature `compression`**（P1）

- 用途：gzip/deflate 压缩（星表分发、FITS 压缩扩展、缓存格式）。**只用纯 Rust 后端**：`flate2 = { version = "1", default-features = false, features = ["rust_backend"] }`（最新 1.1.9，`rust-version=1.67`；`rust_backend` = miniz_oxide，纯 Rust）。
- 默认：关闭。no_std：miniz_oxide 支持 no_std（`flate2` 自身仍使用 std I/O）。所选 `rust_backend` 无 FFI；`zlib` / `zlib-ng` 后端使用 C，`zlib-rs` 是另一个纯 Rust 后端但本项目不选。许可：MIT OR Apache-2.0（miniz_oxide：MIT OR Zlib OR Apache-2.0）。平台：全平台。
- 为何不自己实现：DEFLATE 编解码器是成熟算法，实现正确性成本高。
- 替代项：`gzip`（旧）、`libflate`（纯 Rust 但慢）、`zlib-rs`（新兴，性能好）。理由：flate2 + miniz_oxide 是生态默认的纯 Rust 组合；默认构建仍零额外依赖（feature 关闭）。

#### 3.3.7 FITS 方案：fitsrs / fitsio —— **可选 feature `fits`**（P2），采用 fitsrs =0.4.1

- 需求：CAT-DAT-005/006 的 TAP FITS 星表结果、FRM-BDY 形状模型和多 HDU 流式访问。Gaia DR3 官方 bulk 本身是 ECSV(gzip)，不是 FITS 分片。
- **fitsrs（CDS，纯 Rust）**：官方仓库 `cds-astro/fitsrs`，crates.io 最新发布版 0.4.1，许可 `Apache-2.0 OR MIT`，edition 2018。支持多 HDU、图像、基础 BINTABLE、流式/seek 访问和部分 tiled-image compression；不提供写入器，ASCII 表只暴露原始字节。其 BINTABLE 支持主要为瓦片压缩图像加入，不能未经真实 Gaia TAP FITS 样本验证便声称完整支持星表。
- 版本约束：`fitsrs = "=0.4.1"`；仓库中的 0.4.2 尚未发布且含 git 依赖，不能写入 crates.io 版本约束。feature `fits = ["dep:fitsrs"]`，默认关闭。接入验收必须用 Gaia/FITS BINTABLE 固定样本验证列类型、变长数组、空值、缩放、字节序和截断输入。
- no_std：不支持；活动源码 0 unsafe、无 FFI；许可：Apache-2.0 OR MIT；MSRV：未声明；平台：std 平台及 WASM 纯 Rust 路径。
- **名称陷阱**：`fitrs` 是另一个名称相近但无关的 crate，不得误写为 hyastro 依赖。
- **fitsio**：基于 cfitsio 的成熟绑定，但需要 C 工具链并扩大 FFI 审计面；当前生产基线明确不采用。若未来要求完整 FITS 写入或 fitsrs 未覆盖的扩展，必须另立依赖决策，不得在 `fits` feature 中静默切换实现。
- 为何不自己实现：FITS 卡片、HDU、BINTABLE、填充、字节序和压缩约定复杂；采用 CDS 解析器并由 hyastro 适配层补充领域列映射。

#### 3.3.8 VOTable 方案：votable（CDS）—— **可选 feature `votable`**（P2）

- 需求：CAT-DAT-005 VOTable 流式适配；Gaia TAP 的默认与 gzip 输出均支持 VOTable。
- crate：`votable` 0.7.0（crates.io `repository=https://github.com/cds-astro/cds-votable-rust`，即 CDS 官方；Apache-2.0 OR MIT；edition 2024）。支持 VOTable 1.0–1.6 标签（当前 IVOA Recommendation 为 1.5），XML-TABLEDATA/BINARY/BINARY2 与 JSON/YAML/TOML 互转、MIVOT，以及 StAX 流式读取。
- 版本约束：`votable = "0.7"`。默认：关闭。no_std：不支持；无 FFI，但源码有约 19 处 UTF-8 快速路径的非 FFI `unsafe`。
- 为何不自己实现：VOTable 协议（FIELD/DATA/BINARY 编解码、arraysize/precision）繁琐。
- 替代项：`quick-xml` 手写（重复造轮子）；`votable` 的竞品暂无。**注意**：CDS README 自述 "not yet as clean and documented as I would like"、API 可能调整（0.x）——锁版本并适配层薄；接入时机为 P2。
- 决策：P2 可选 feature；若 P2 评审认为 API 不稳定，可降级为"内部验证工具"或推迟（保持 `votable` feature 名占位？否——不设占位；当期结论：可选 feature，实施时以 0.7 锁定）。

#### 3.3.9 Arrow / Parquet 方案：arrow + parquet —— **可选 feature `parquet`**（P1/P2）

- 需求：把 Gaia 官方 ECSV/TAP 结果自行转换为 Parquet，支持大规模列式访问、批次传播和交叉匹配。ESA bulk 与 TAP capabilities 均不提供官方 Parquet；第三方 Parquet 数据集必须记录转换来源和校验信息。
- 版本：arrow 59.2.0 / parquet 59.2.0（Apache-2.0，`rust-version=1.85`，edition 2024）；与 nyx-space 的 arrow/parquet 59 同代。
- 版本约束：`arrow = "59"`、`parquet = "59"`；`parquet` 的 `arrow` feature 提供批次迭代读取。默认：关闭。
- no_std：不支持。arrow/parquet 内部存在非 FFI unsafe；parquet 默认 codec 集包含 `zstd`，会经 `zstd-sys` 引入 C，启用时必须在 feature 与供应链审计中明确。许可：Apache-2.0。平台：std。
- 为何不自己实现：Parquet 编码（D rem、RLE、delta、page 压缩、schema 嵌套）是重型协议。
- 替代项：`polars`（更高层但依赖更重、API 域外）；`datafusion`（查询引擎，过重）；经 csv 间接（无列式性能）。理由：直接 arrow/parquet 面最小；feature 关闭时零影响。
- **体积/编译时间警告**：arrow/parquet 会显著增加编译时间（ANISE 注释：启用 Arrow/Polars 显著增加编译时间，`anise/Cargo.toml` `validation` 特性注释附近 [观察]）；作为独立 feature 隔离（F-FEAT-001 加法性）。

#### 3.3.10 HEALPix / MOC 方案：cdshealpix + moc —— **可选 feature `healpix` / `moc`**（P2）

- 需求：MATH-SPH-006 可选 HEALPix 索引/邻域/圆锥检索/多分辨率覆盖；P2 星表交叉匹配与 MOC 区域查询。
- `cdshealpix` 0.9.1（crates.io `repository=https://github.com/cds-astro/cds-healpix-rust`；Apache-2.0 OR MIT；`rust-version=1.81`）：HEALPix 细分（nested/ring）、单元邻域、锥形检索、UNIQ 编号；纯 Rust。
- `moc` 0.19.2（CDS cds-moc-rust；Apache-2.0 OR MIT；无 rust-version 声明）：多阶覆盖图（MOC 2.0），依赖 cdshealpix；纯 Rust。
- 版本约束：`cdshealpix = "0.9"`、`moc = "0.19"`（moc feature 隐式依赖 healpix，故 `healpix` feature 是 `moc` 的前置或可独立）。默认：关闭。
- no_std：不支持（std）。unsafe/FFI：无（纯 Rust）。平台：全平台（含 wasm）。
- 为何不自己实现：HEALPix 编号/邻域算法有几何细节（nested 位运算、UNIQ 编解码），CDS 实现活跃且被 MOCPy/Aladin 生态验证。
- 替代项：`healpix`（绑定 healpix C++ 库——FFI，明确不采用）；自研（无必要）。理由：CDS 纯 Rust 实现是唯一活跃选项。

#### 3.3.11 SGP4/TLE 方案：sgp4 —— **可选 feature `sgp4`**（P2）

- `sgp4` 2.4.0（`repository=https://github.com/neuromorphicsystems/sgp4`；MIT；纯 Rust）：实现 Vallado 近地/深空 SGP4、TLE 解析和可选 serde OMM 解析；传播结果是 TEME 语义，**不提供 TEME→GCRS/ITRS 转换**，该转换由 hyastro 参考系模块承担。
- 版本约束：`sgp4 = { version = "2", default-features = false, features = ["std"] }`；仅 OMM 工作流额外启用 `serde`。默认关闭。
- no_std：上游 TLE 解析和传播支持无 std、无 alloc，但须用其 `libm` feature；hyastro 当前 `sgp4` 适配器属于 std-only P2，`no_std` 子集不包含它。unsafe/FFI：无。MSRV：未声明（edition 2021）。
- 为何不自己实现：SGP4 深空共振/近地点摄动的实现与验证需要官方测试集（sgp4 crate 内嵌 Vallado 验证数据）。
- 替代项：`satkit`（更全但含 FFI 与更多域）；`gregdavies/sgp4`（停更）；自研（不推荐）。理由：纯 Rust、MIT、Vallado 系实现。
- **注意**：ANISE 明确不读 SPK Type 10（TLE 段，README "Please don't use TLEs"）——hyastro 的 TLE 需求由 sgp4 独立处理，与 ANISE 无冲突。

#### 3.3.12 压缩格式综合：flate2 / miniz_oxide / zstd / bzip2

| crate | 版本 | 许可 | MSRV | FFI | 决策 |
|---|---|---|---|---|---|
| flate2（rust_backend） | 1.1.9 | MIT OR Apache-2.0 | 1.67 | 否（miniz_oxide 后端） | **可选 feature `compression`**（P1，见 3.3.6） |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | 未声明 | 否 | 经 flate2 传递；不直接依赖 |
| zstd | 0.13.3 | MIT | 1.64 | 是（zstd-sys 捆绑 C） | **不直接依赖**：仅在 parquet 文件确需 zstd codec 时由 `parquet` feature 传递启用并接受其 FFI |
| bzip2 | 0.6.1 | MIT OR Apache-2.0 | 1.82 | 是（libbz2-sys） | **明确不采用**：低频格式、FFI、维护一般 |

### 3.4 地理 / 大气 / 轨道

#### 3.4.1 geographiclib-rs —— **可选 feature `geodesy`**（P1）

- 用途：WGS84 或用户给定椭球上的测地线正解、反解，以及多边形周长/面积。SOFA 的 `eform`/`gc2gd`/`gd2gc` 负责椭球常数与地心/大地坐标互转，两者职责不重叠。
- 版本：`geographiclib-rs` 0.2.7（georust；MIT；`rust-version=1.70`；纯 Rust），实现 Karney 测地线算法的子集。
- 版本约束：`geographiclib-rs = { version = "0.2", default-features = false }`；仅需要高精度多边形面积时启用其默认 `accurate` 能力。默认关闭。no_std：不支持。unsafe/FFI：无。
- 为何不自己实现：Karney 正反测地线在对跖与近对跖情形的稳定实现和验证成本高。
- 能力边界：该 crate **不提供 UTM 投影，也不提供 ECEF↔大地坐标转换**。ECEF↔大地坐标由 sofars/hyastro 完成；未来地图投影需求另行选择投影库。
- 替代项：简单 Vincenty 实现在近对跖处可能不收敛；C++ GeographicLib 绑定引入 FFI。纯 Rust geographiclib-rs 是当前最小选择。

#### 3.4.2 SGP4/TLE —— 见 3.3.11。

### 3.5 时区 / 数值 / 随机

#### 3.5.1 jiff —— **可选 feature `timezone`**（P2）

- 用途：IANA 时区互操作（TIME-ZONE-001/002/003：核心只承诺 UTC 与固定偏移；IANA 时区为可选适配器；DST 歧义策略由调用者选择）。
- 版本：`jiff` 0.2.35（Unlicense OR MIT；`rust-version=1.70`）；纯 Rust tz 数据库解析（tzdb 数据可选内嵌）。
- 版本约束：`jiff = "0.2"`（可选）。默认：关闭。
- no_std：不支持（std）。unsafe/FFI：无。平台：全平台（tz 数据可打包）。
- 为何不自己实现：IANA tz 规则（TZif 二进制、DST 变迁、未来规则）解析成熟且易错。
- 替代项：`chrono-tz`（静态编译期 tz 数据，体积大、更新时间靠发版）；`tz-rs`（纯 Rust，成熟度低）；`time` 的 tz 支持（0.3 起 experimental）。理由：jiff 是当前纯 Rust 时区实现的事实标准；与 hifitime 的映射走 unix 秒（注意 hifitime 闰秒语义 vs jiff POSIX 语义，转换层显式处理）。
- **不采用**：`chrono`（时间语义与 hifitime 冲突，引入第二套时间模型）。

#### 3.5.2 求根 / 优化：brentroot、argmin、roots —— **明确不采用**，自研

- 需求：PRD MATH-NUM-002/003（带括区 Brent 类求根、二分、割线、Newton 混合；有界一维极值、根去重、解包裹）——这是 hyastro 事件模块（升落/中天/食）的核心原语，必须可审计、返回收敛状态/残差/迭代次数（MATH-NUM-005），外部通用库难以满足领域语义（角度解包裹、周期扫描、物理单位容差）。
- crates.io 现状：`brentroot` 查询失败（2026-08-05，API 无结果，疑已下架/改名）；`argmin` 0.11.0（通用优化框架，依赖重，面向非线性最优化而非求根）；`roots` 0.0.8（2018 年起低维护 [推断]，仅多项式）。
- 决策：自研（hyastro `math::root` 模块，约 300-500 行）；参考 ANISE `analysis::utils::brent_solver`（`anise/src/analysis/mod.rs:34` 导出）的接口形态。
- 替代项：`nalgebra` 的 `Brent`？无——nalgebra 不提供求根器；`argmin` 过度设计。

#### 3.5.3 随机 / 统计：rand / rand_chacha / rand_distr —— **仅开发/验证**

- 用途（仅 dev）：蒙特卡洛验证辅助（MATH-NUM-007）、协方差采样测试、fuzz 种子、确定性伪随机（ChaCha20Rng）。
- 版本：`rand = "0.10"`（最新 0.10.2）、`rand_chacha = "0.10"`（0.10.0）、`rand_distr = "0.6"`（0.6.0）；均 MIT OR Apache-2.0，`rust-version=1.85`（edition 2024，与 ANISE dev-deps 的 rand 0.10/rand_pcg 0.10 同代）。
- 生产路径：**不依赖 rand**（核心计算确定性；PRD 无随机数需求）。若未来需要运行时随机（事件模拟），再评估 `rand` feature（默认仍关）。

### 3.6 验证与参考库

#### 3.6.1 rsofa —— **仅开发/验证**（SOFA C 逐位对照 oracle）

- 0.5.0（2023-12-21 最后提交）；MIT + SOFA 条款；bundles SOFA C 2023-10-11（248 个 .c）；bindgen+cc 构建、全 unsafe 裸绑定、无测试覆盖（LIBRARY_RESEARCH 3.2）。
- 角色：与 sofars 同对照 SOFA 2023-10-11，二者互验（F-TEST-003"SOFA/ERFA 官方验证向量"的 C 原生路径）；只进验证 crate（如 `hyastro-validation`），不进入生产依赖；构建需 C 工具链——仅在 CI 有编译器时启用（CODE_STANDARDS 15 节"C/Fortran 构建依赖必须在支持平台 CI 实际编译"）。

#### 3.6.2 erfa-sys / erfa（cjordan/rust-erfa）—— **明确不采用**

- erfa-sys 0.2.1（MPL-2.0 + ERFA BSD-3 条款；`links="erfa"`；默认路径需系统 liberfa，`static` 特性需检出子模块——本地克隆子模块未检出，build.rs 对空目录直接 panic，`ref/erfa-sys/erfa-sys/build.rs:58-67`）；erfa（纯 Rust crate）仅 57 函数、README 自认 incomplete、2022-11 停更。
- 理由：停更、构建前置条件（系统库）、`links` 独占（同图内不能再链第二份 ERFA）、与 sofars 算法同源（SOFA↔ERFA）造成重复维护。若未来必须对接 ERFA C 生态，独立评估（F-FEAT-003 预留 `erfa` feature 名，当前不启用）。

#### 3.6.3 novas（Mubelotix/novas，NOVAS C3.1）—— **仅开发/验证**

- 0.1.3；`links="novas_c31"`；bundles USNO NOVAS C3.1（2011）；parity 自动化仅覆盖 4 函数；wasm 需 emscripten；**许可声明冲突**：Cargo.toml=MIT、README=GPLv3-only、仓库无 LICENSE、上游"无许可要求"（LIBRARY_RESEARCH 3.7/2.4）。
- 角色：与 sofars/ANISE 独立实现交叉验证（F-TEST-007"NOVAS/SOFA/ERFA 独立差分路径"），用于视位置/地球定向差分。**前提**：接入前向作者确认包装层许可；只进验证 crate。

#### 3.6.4 nyx-space —— **明确不采用**

- 2.5.0；AGPL-3.0-or-later + `premium` 双许可（默认开 `premium`，营利门槛）；依赖树重（arrow/parquet/dhall/hyperdual）；领域为飞行力学/定轨，与 hyastro 天文计算域错位（LIBRARY_RESEARCH 3.5/6.3）。
- 保留价值：作为"hifitime + anise 集成模式"与 RK 积分器/事件检测的架构参考（代码阅读蓝本，不进依赖）。

#### 3.6.5 rust-astro（astro 2.0.0）—— **明确不采用**（移植蓝本）

- 2018-07 停更、edition 2015、零依赖、MIT；唯一覆盖天文事件（升落/中天/月相/二分点，`transit.rs`）与 VSOP87D/ELP-2000/82 的现成实现。
- 角色：事件算法与 VSOP87D 系数的**移植蓝本**（MIT 许可允许）；不进入依赖图（版本/API 与现代 Rust 不兼容）。

#### 3.6.6 celestial-eop-data（gaker）—— **明确不采用**

- crates.io 元数据与仓库（`gaker/celestial-eop-data`）：Apache-2.0 仓库（GitHub license 字段）＋ 仓库 LICENSE-APACHE/LICENSE-MIT 双文件、Cargo.toml `license = "MIT OR Apache-2.0"`；最新发布 0.1.x（主代理提供 crates.io 当前 0.1.12；仓库 HEAD 为 0.1.21）。
- 机制核查 [观察]：`build.rs` 在**构建期**解析仓库内 `data/eopc04.1962-now` 与 `data/finals2000A.all`（IERS 官方格式文本）并压缩为 zstd 二进制内嵌——**构建期不联网**；数据由维护者 GitHub Action 每周更新（README 自述）；运行时零联网（懒加载内嵌数据）；来源固定为 IERS C04 + finals2000A。
- 为什么不采用（核心）：(1) `EopEntry` 丢失观测/预报标志，且构建期 finals 解析器把缺失 LOD、`dX/dY` 写成 `0.0`，无法执行显式来源接纳策略或区分“缺失”和“真实零”；(2) 每周更新的发布节奏导致版本漂移快、可复现性差（类似 ANISE 的 earth_latest_high_prec.bpc 问题）；(3) 自定义二进制容器（EOP1 魔数 + zstd）是外来格式，需转换层；(4) `build.rs` 依赖外部 `date` 命令（`chrono_free_utc_date` 调用 `date -u`），Windows 构建不可移植 [推断]。
- 结论：hyastro 自有 `EopProvider` 接口（PRD 4.2）+ `finals2000A.all`/`C04` 解析器（自有实现，IERS 原始格式）为规范；celestial-eop-data **不进入依赖**。其数据可作为开发验证期的参考对照源（人工核对，非自动依赖）。

#### 3.6.7 celestial / siderust 聚合天文 crate —— **明确不采用**

- `celestial`（gaker）0.1.0（crates.io 占位描述 "Placeholder for in-progress project"；仓库 2026-06 活跃，Apache-2.0）；`siderust` 组织仓库（siderust/qtty/tempoch/cheby/keplerian/principia/optica/gaussian/affn 等）多为 AGPL-3.0。
- 理由：(1) 聚合 crate 自带完整领域模型（时间/参考系/坐标/天体类型），引入即形成**第二套类型体系**，与 0.2 原则 5 冲突；(2) `celestial` 尚处 0.1 占位、API 不稳定；(3) siderust 系 AGPL 对商用闭源是硬约束；(4) 与 hifitime/sofars/ANISE 已定内核功能重叠。hyastro 只从这些项目吸收算法思想，不依赖其类型。


## 4. 开发依赖（仅 `[dev-dependencies]` / 验证 crate）

原则：开发依赖不进生产构建；重型或 FFI 项（rsofa/novas/rust-spice）只放独立验证 crate（如 `hyastro-validation`）并由 CI 显式启用，防止污染常规 `cargo test` 默认路径。

### 4.1 断言与性质测试

| crate | 版本（最新） | 用途 | 取舍说明 |
|---|---|---|---|
| approx | 0.5.1（Apache-2.0） | 浮点相对/绝对容差断言（`assert_relative_eq!` 等） | 对照 SOFA 官方验证向量（F-TEST-003）与往返性质测试（F-TEST-008）的标配 |
| proptest | 1.11.0（MIT OR Apache-2.0，MSRV 1.85） | 属性测试：角度规范化、跨零解包裹、负年份、闰秒边界、极点/对跖（F-TEST-009） | 与 rstest 互补：proptest 面向性质不变量，rstest 面向固定用例矩阵；二者都保留（用途不同） |
| rstest | 0.26.1（MIT OR Apache-2.0，MSRV 1.70） | 参数化测试 + fixture | ANISE/hifitime 生态同款（anise dev-deps rstest 0.26.1）；取舍：比 `#[test]` + 循环更可读、比 proptest 更精确控制输入；固定验证向量用 rstest，性质探索用 proptest |
| trybuild | 1.0.120（MIT OR Apache-2.0，MSRV 1.88） | 编译期错误诊断测试（类型系统拒绝误用：跨参考系相加、UTC/TT 混用、点/向量混淆） | 强类型承诺的回归护栏（F-TEST-008 的编译期部分）；ANISE 亦用 `__ui_tests` cfg 跑 compile_fail |
| serde_json | 1（dev） | 序列化往返测试 | 仅 dev；若启用 `serde` feature 则作为测试对照 |

### 4.2 基准与分配计数

| crate | 版本 | 用途 | 取舍 |
|---|---|---|---|
| criterion | 0.8.2（Apache-2.0 OR MIT，MSRV 1.86） | 时间基准（F-PERF-004：时间/地球定向/SPK/星表/事件） | ANISE 同款 0.8；稳定统计输出 |
| iai-callgrind | 0.16.1（Apache-2.0 OR MIT，MSRV 1.74） | 指令级基准 + 分配计数回归（F-PERF-005） | 与 criterion 互补：时间 vs 指令/堆分配；Linux + valgrind 前置 |
| 取舍原则 | — | 时间与分配两类指标分开 | 基准默认不跑（`#[ignore]` 或独立 bench），避免拖慢 CI |

### 4.3 Fuzz：cargo-fuzz / arbitrary / libfuzzer-sys

- 工具：`cargo-fuzz` 0.13.2（仅工具链，nightly 组件）；`arbitrary` 1.4.2（MIT OR Apache-2.0，MSRV 1.63，derive 支持）+ `libfuzzer-sys` 0.4（dev）。
- 覆盖（F-SAFE-006：BSP、FITS、VOTable、CSV、IERS 解析 fuzz）：复用 ANISE 模式（25 个目标：`parse_spk`/`parse_bpc`/`kpl_parse_*`/`rotation_*`/`load_from_bytes` 等，`ref/anise/anise/fuzz/fuzz_targets/`）；hyastro 新增 FITS/VOTable/CSV/IERS 解析目标与自有格式解析目标。
- 决策：fuzz 目标为独立 crate（`hyastro-fuzz`），nightly-only，不进 workspace 默认成员。

### 4.4 临时文件与随机

- `tempfile` 3.27.0（MIT OR Apache-2.0，MSRV 1.63）：测试用临时数据文件（SPK 快照、EOP 样例、星表小样本）。
- `rand` 0.10.2 + `rand_chacha` 0.10.0 + `rand_distr` 0.6.0（见 3.5.3）：蒙特卡洛验证辅助、协方差采样、确定性种子（ChaCha20Rng）。
- `rand_pcg` 0.10.0：与 hifitime/ANISE dev 生态同款（如需兼容其测试种子）。

### 4.5 C 参考 oracle（仅验证 crate）

| oracle | 版本 | 角色 | 前置条件 |
|---|---|---|---|
| rsofa | 0.5.0 | SOFA C 2023-10-11 逐位对照（与 sofars 互验） | C 编译器 + bindgen（CI Linux/macOS 启；Windows 跳过） |
| rust-spice | 0.7.8（`rust-spice`，GitHub GregoireHENRY/rust-spice；anise 用 0.7.6） | CSPICE 差分（F-TEST-005：`spkezr`/`pxform` 对照；ANISE 验证模式同款） | CSPICE 源码构建（CI 安装，见 ANISE `dev-env-setup.sh` 模式） |
| novas | 0.1.3 | NOVAS C3.1 独立差分（F-TEST-007） | 许可先澄清（3.6.3）；CC 编译 |
| ERFA（系统 liberfa） | 由 CI 安装 | 可选第三对照（SOFA 系） | 不进 Cargo 清单 |

策略（CODE_STANDARDS 15 节）：多个 oracle 只用于**测试对照**（"除非一个仅在测试中作为 oracle"）；生产依赖图绝不包含 C/FFI 数值内核。

## 5. 工具链（仅 CI / 发布流程，不进 Cargo 清单）

| 工具 | 版本（2026-08-05） | 用途 | 备注 |
|---|---|---|---|
| cargo-deny | 0.20.2 | 许可/依赖审计（F-REL-002）：禁止清单（第 8.2 节）、双许可校验、来源审计 | CI 必跑；配置 `deny.toml` |
| cargo-audit | 0.22.2 | 已知漏洞扫描（RustSec advisory DB） | CI 定时 + PR |
| cargo-semver-checks | 0.50.0 | 破坏性 API 变更检查（SemVer 门禁） | 针对 ANISE/hifitime/sofars 升级与 hyastro 自身发布 |
| cargo-msrv | 0.19.3 | MSRV 测定与回归（避免依赖被动抬升） | 与 2.3 节 MSRV 记账配合 |
| rustfmt / Clippy | rustup 组件 | 格式与 lint 门禁（-D warnings） | 与 ANISE CI 同模式 |
| Miri | rustup 组件（nightly） | unsafe 检测（本项目 unsafe 面极小，但 ANISE 适配层的 zerocopy/bytes 路径可验） | 周期运行 |
| Kani | rustup 组件 | 模型检查（hifitime 先例：`.github/workflows/formal_verification.yml`；hyastro 时间/历法核心可复刻） | 可选强化 |
| cargo-llvm-cov | 0.6.x | 覆盖率门禁 | ANISE 同款 |
| cargo-fuzz | 0.13.2 | fuzz 运行器（见 4.3） | nightly |
| cargo-deny 的 `bans` | — | 传递依赖白名单与 `#![forbid]` 类策略 | 见 8.2 |


## 6. Cargo feature 映射与依赖图

### 6.1 Feature 清单（主 crate 建议形态）

遵循 FEATURES.md F-FEAT-001/003 与 CODE_STANDARDS 15 节：feature 名 = 依赖或能力名，`dep:` 绑定，全部加法性，默认只开 P0 最小集。

| feature | 启用依赖 | 默认 | 说明 |
|---|---|---|---|
| `std` | `hifitime/std` + `dep:sofars` | 开 | 默认完整数值路径；`--no-default-features` 排除不支持 no_std 的 sofars，只保留 math/纯时间表示子集 |
| `serde` | `dep:serde`（derive） | 关 | hyastro 自有类型序列化（P1） |
| `rayon` | `dep:rayon` | 关 | 批处理/并行查询（P1）；仅经 hyastro 自有并行原语 |
| `anise` | `dep:anise`（default-features=false） | 关 | SPICE/DAF/SPK/PCK 后端（第 2 节）；**不含** anise 的 metaload/analysis/embed_ephem |
| `logging` | `dep:tracing` + `dep:tracing-log` | 关 | 结构化日志 + `log` 桥接 |
| `text-parsing` | `dep:winnow` | 关 | 自研文本格式解析（P1/P2） |
| `catalog-csv` | `dep:csv` | 关 | Gaia/星表 CSV 流式适配（P1） |
| `geodesy` | `dep:geographiclib-rs` | 关 | 测地线正反解与多边形测量（P1） |
| `vsop87` | `dep:vsop87` | 关 | 太阳/行星低精度历表（P1） |
| `compression` | `dep:flate2`（rust_backend） | 关 | gzip/deflate（P1，纯 Rust） |
| `integrity` | `dep:sha2` | 关 | 数据校验和（P1） |
| `fits` | `dep:fitsrs`（=0.4.1） | 关 | FITS 读取（P2） |
| `votable` | `dep:votable` | 关 | VOTable（P2） |
| `parquet` | `dep:arrow` + `dep:parquet` | 关 | 列式星表（P1/P2，重依赖，独立隔离） |
| `healpix` | `dep:cdshealpix` | 关 | HEALPix（P2） |
| `moc` | `dep:moc`（含 healpix） | 关 | MOC（P2） |
| `sgp4` | `dep:sgp4` | 关 | SGP4/TLE（P2） |
| `timezone` | `dep:jiff` | 关 | IANA 时区适配（P2） |

加法性规则：`anise` 不得隐式开启 `parquet`/`rayon`；`parquet` 不得隐式开启 `anise`；`logging` 不改变计算结果（只影响诊断）；默认仅启用 `std`。`no_std` 不是 feature，而是 `--no-default-features` 构建模式。当前所有可选适配器 feature 均显式包含 `std`，禁止组成“feature 可开但无法编译”的 no_std 组合。

### 6.2 依赖图（mermaid）

```mermaid
flowchart LR
    subgraph P0-默认内核（纯Rust、离线）
        HF[hifitime 4.3] --> HT[NUM num-traits+libm]
        SF[sofars =0.6.1]
        TE[thiserror 2]
        HA[hyastro math/time 自研: Vec3/Mat3/Quat/量类型/求根]
        HA --> SF
        HA --> HT
    end
    subgraph 可选-feature
        AN[anise 0.10.4 default-features=false] -->|传递| NAL[nalgebra =0.35]
        AN -->|传递| HF
        AN -->|传递| SF
        RY[rayon 1]
        SD[serde 1]
        TR[tracing + tracing-log]
        CSV[csv 1] / WN[winnow 1]
        GEO[geographiclib-rs 0.2]
        VS[vsop87 3]
        FL[flate2 1 rust_backend]
        SH[sha2 0.11]
        FITS[fitsrs =0.4.1] / VOT[votable 0.7]
        AR[arrow+parquet 59]
        HP[cdshealpix 0.9 + moc 0.19]
        SG[sgp4 2] / JF[jiff 0.2]
    end
    subgraph 开发验证（不进生产）
        RSO[rsofa 0.5] / NV[novas 0.1.3] / RSP[rust-spice 0.7.8] --> VAL[hyastro-validation crate]
        AP[approx] / PT[proptest] / RT[rstest] / TB[trybuild] / CR[criterion] / IA[iai-callgrind]
        FUZZ[cargo-fuzz + arbitrary] / TM[tempfile] / RD[rand系]
    end
    HA -. std路径数值核对 .-> SF
    HA -. 差分 .-> RSO
    AN -. 适配层转换 .-> HA
```

要点：
- P0 内核（`math`/`time` 自研 + hifitime + sofars + thiserror + libm）是唯一默认构建；hifitime 与 sofars 之间存在既有 dev 交叉验证链（hifitime 以 sofars 校验 TDB/TCG），hyastro 继承该模式。
- ANISE 分支只出现在 `anise` feature；`nalgebra` 仅为 ANISE 传递（统一 =0.35）。
- 所有 C/FFI oracle（rsofa/novas/rust-spice）汇聚在 `hyastro-validation` 独立 crate，默认 workspace 成员中排除或 feature 门控。

## 7. 许可证矩阵与供应链策略

### 7.1 许可证矩阵（2026-08-05 crates.io 元数据）

| crate | 许可 | 传染性 | 备注 |
|---|---|---|---|
| hifitime | MPL-2.0 | 文件级 | 不修改源码即不传染 |
| anise | MPL-2.0 | 文件级 | 同上；修改须回馈（2.2） |
| sofars | MIT + SOFA 条款 | 无（有派生义务） | 派生命名禁 `iau`/`sofa` 前缀、再分发声明差异、出版物致谢 |
| rsofa（验证） | MIT + SOFA 条款 | 无 | 分发未改动 SOFA C，同款义务 |
| novas（验证） | 包装层 MIT vs GPLv3 冲突 | 待澄清 | 上游无许可要求；澄清前仅内部验证 |
| thiserror / num-traits / libm / sha2 / winnow / csv / rand 系 / approx / proptest / trybuild / rstest / criterion / iai-callgrind / arbitrary / tempfile / serde / tracing-log / rayon / flate2 / sgp4 | MIT OR Apache-2.0（libm 纯 MIT、winnow 纯 MIT、csv Unlicense/MIT） | 无 | — |
| tracing | MIT | 无 | — |
| geographiclib-rs | MIT | 无 | Karney 算法注明出处 |
| vsop87 | Apache-2.0 | 无 | VSOP87 系数为 IAU/公开数据 |
| cdshealpix / moc / votable | Apache-2.0 OR MIT | 无 | — |
| fitsrs =0.4.1 | Apache-2.0 OR MIT | 无 | CDS 官方纯 Rust解析器；锁定 0.4.1 |
| arrow / parquet | Apache-2.0 | 无 | — |
| jiff | Unlicense OR MIT | 无 | — |
| zstd（parquet 传递） | MIT（zstd-sys 捆绑 C，BSD-3 系） | 无 | 仅 parquet codec 明确启用时进入依赖图 |
| nyx-space | AGPL-3.0-or-later + premium | **强传染** | 禁止 |
| celestial / siderust 系 | Apache-2.0 / AGPL-3.0 | AGPL 项强传染 | 禁止 |
| erfa-sys / erfa | MPL-2.0 + ERFA BSD-3 | 文件级 | 不采用（其他理由） |
| rust-astro | MIT | 无 | 仅蓝本（不依赖） |
| celestial-eop-data | MIT OR Apache-2.0 | 无 | 不采用（数据模型） |

合规动作：`cargo-deny` 配置 `deny.toml` 纳入上表全部许可白名单；`LICENSES/` 归档 MPL-2.0、SOFA 条款、Apache-2.0、MIT 全文（分发要求，CODE_STANDARDS 15 节）。

### 7.2 供应链 / 版本锁定策略

1. **生产依赖锁定**：`Cargo.lock` 提交入库（库项目同样提交，供审计）；semver 范围见各条目；sofars/anise/nalgebra/der/zerocopy/tabled 用精确或窄范围（`=0.6.1`、`=0.35`、`0.7`、`0.8`、`=0.21`）锁定快迭代上游。
2. **上游版本台账**（与 LIBRARY_RESEARCH 7.4 衔接）：SOFA 2023-10-11 ↔ sofars 0.6.1 / rsofa 0.5.0；ERFA 子模块 eb4c95df（未采用）；NOVAS C3.1 ↔ novas 0.1.3；DE 系列（405/421/430/440/440s）↔ 历表后端参数；hifitime 4.3.x；anise 0.10.4。
3. **数据下载策略**（核心离线，PRD 4.2）：
   - hyastro 数据工具（独立命令/脚本，非库行为）下载 SPK/PCK/EOP/星表：**必须 HTTPS**（规避 ANISE metaload 的 http:// 端点风险，见 2.9）、固定 URL + CRC32/sha256 校验、记录数据版本/日期/来源；
   - 版本快照化：`de440s.bsp`、`pck11.pca`、`moon_fk_de440.epa`、`moon_pa_de440_200625.bpc`（ANISE Default 集，CRC32 已给出）、`earth_latest_high_prec.bpc` 改为**日期戳固定版**（不追每日更新）；
   - EOP：仓库固定 `2026-08-06` 的 IERS EOP 20u24 C04 与 finals2000A 原始快照，URL、SHA-256、记录数和有效列边界记录于 `data/eop/SOURCES.toml`；`IersC04` / `IersFinals2000A` 保留空列和内部来源标记，`try_samples_in` 要求调用者显式选择覆盖区间及 `FinalOnly` / `ObservedOrFinal` / `IncludePredicted` 接纳策略；
   - 构建期：**禁止**任何 build.rs 联网（anise 的 `embed_ephem` 特征模式不采用）。
4. **依赖审计门禁**：cargo-audit（漏洞）+ cargo-deny（许可/bans）+ cargo-semver-checks（升级破坏检测）+ cargo-msrv（MSRV 回归）四件套进 CI；参考源码 commit（ref/）只用于调研与差分，不成为构建输入（CODE_STANDARDS 15 节）。

## 8. 禁止重复类型体系与禁止依赖清单

### 8.1 禁止重复类型体系（单一事实来源）

| 领域 | 规范类型（hyastro 自有） | 禁止混入 | 例外（适配层内部） |
|---|---|---|---|
| 时间 | `Instant`/`Duration`/`TimeScale`/`Date`/`DateTime`/`Epoch`（包装 hifitime 或自研） | chrono/jiff 时间模型、裸 f64 JD | hifitime `Epoch` 仅在时间适配器内部 |
| 参考系/帧 | `Frame`/`ReferenceSystem`/`ReferenceFrame`/`Origin`（自有） | ANISE `Frame`、spice 的 NAIF 裸 ID 语义 | ANISE 适配器内 NAIF ID 映射 |
| 单位/量 | 自有量类型 + 版本化常数组 | uom/nalgebra 泛型量、裸数 | 无 |
| 线性代数 | 自有 `Vec3`/`Mat3`/`Quaternion`/`StateTransform` | nalgebra 类型出现在公开面 | ANISE 内部（仅适配器） |
| 历表状态 | 自有 `State`/`Ephemeris` 接口 | ANISE `CartesianState`/`Orbit` | ANISE 适配器 |
| 错误 | thiserror 枚举（`#[non_exhaustive]`） | snafu/anyhow 类型出现在公开面 | anise snafu 错误经 `map_err` 转换 |
| 恒星时/地球定向 | sofars 为数值内核、hyastro 封装 | 第二套恒星时实现 | ANISE 动态帧（IAU 帧模型）仅作 SPICE 帧 |

### 8.2 禁止依赖清单（cargo-deny bans 落地）

| 包 | 原因 |
|---|---|
| `nyx-space` | AGPL-3.0-or-later + premium 双许可 |
| `erfa-sys` / `erfa` | 停更、系统库前置、`links="erfa"` 独占 |
| `astro`（rust-astro） | 停更、edition 2015 |
| `novas` | 许可声明冲突（澄清前禁止入生产；仅验证 crate 人工评审后使用） |
| `uom` | 领域语义不匹配 |
| `nalgebra`（直接依赖） | 抬升 MSRV、无天文语义；仅允许传递（`bans: allow-multiple-versions` 控制为 0.35 单版本） |
| `chrono` | 第二套时间模型 |
| `argmin` / `roots` / `brentroot` | 求根自研 |
| `bzip2` | FFI + 低频 |
| `fitsio`（生产） | cfitsio FFI；当前基线不采用 |
| `fitrs` | 与 CDS 官方 `fitsrs` 名称相近但无关，禁止误加 |
| `celestial` / `celestial-eop-data` / `siderust` 系 | 第二套领域模型 / EOP 数据模型不达标 / AGPL |
| `polars` / `datafusion` | 查询引擎域外、依赖重 |
| `cspice`/`spice`（生产） | FFI + 全局状态（仅验证 crate 使用 rust-spice） |

## 9. P0 / P1 / P2 集合与最终直接依赖总表

### 9.1 最小 P0 直接依赖集（默认构建；纯 Rust、离线、低依赖）

```toml
[features]
default = ["std"]
std = ["hifitime/std", "dep:sofars"]

[dependencies]
hifitime = { version = "4.3", default-features = false }                      # 时间内核
sofars    = { version = "=0.6.1", optional = true }                           # std 天文算法内核
thiserror = "2"                                                              # 错误
libm      = "0.2"                                                            # std/no_std 统一浮点函数
```

P0 自研（不依赖）：`Vec3`/`Mat3`/`Quaternion`/`StateTransform`、量类型与常数组、求根/极值/插值、EOP 数据模型与 `EopProvider`、事件计算、SOFA 缺失的 17 个工具函数补齐。

### 9.2 P1 扩展集（可选 feature，默认关）

`serde`（serde 1 + derive）、`rayon`（1）、`anise`（0.10.4，default-features=false）、`logging`（tracing 0.1 + tracing-log 0.2）、`catalog-csv`（csv 1）、`text-parsing`（winnow 1）、`geodesy`（geographiclib-rs 0.2）、`vsop87`（vsop87 3）、`compression`（flate2 1 rust_backend）、`integrity`（sha2 0.11）。开发侧：criterion、iai-callgrind、proptest、trybuild、rust-spice 差分、rsofa 对照。

### 9.3 P2 专门集（可选 feature，默认关）

`fits`（fitsrs =0.4.1）、`votable`（votable 0.7）、`parquet`（arrow+parquet 59）、`healpix`（cdshealpix 0.9）、`moc`（moc 0.19）、`sgp4`（sgp4 2）、`timezone`（jiff 0.2），以及通过 `--no-default-features` 验证的 `no_std` 内核。开发侧：novas 差分（许可澄清后）、cargo-fuzz 目标、Kani 核心证明。

### 9.4 最终直接依赖总表（全部状态；无重复、无遗漏）

**确定采用（4）**：hifitime、sofars、thiserror、libm
**可选 feature（19）**：serde、rayon、anise、tracing、tracing-log、winnow、csv、vsop87、geographiclib-rs、flate2、sha2、fitsrs(=0.4.1)、votable、arrow、parquet、cdshealpix、moc、sgp4、jiff
**仅开发/验证（17）**：approx、proptest、rstest、trybuild、serde_json、criterion、iai-callgrind、arbitrary、libfuzzer-sys、tempfile、rand、rand_chacha、rand_distr、rand_pcg、rsofa、rust-spice、novas
**仅工具链（12）**：cargo-deny、cargo-audit、cargo-semver-checks、cargo-msrv、cargo-fuzz、cargo-llvm-cov、rustfmt、Clippy、Miri、Kani、rustup nightly 组件、valgrind（iai 前置）
**明确不采用（29）**：bitflags、nalgebra（直接）、uom、num-traits（直接）、memmap2、zerocopy、bytes、der、crc32fast、indexmap、tabled、const_format、snafu、log、erfa-sys、erfa、nyx-space、rust-astro、brentroot、argmin、roots、bzip2、fitsio、fitrs、celestial、celestial-eop-data、polars、chrono（直接）、zstd（直接）

> 注：`num-traits`、`memmap2`、`zerocopy`、`bytes`、`der`、`crc32fast`、`indexmap`、`tabled`、`const_format`、`snafu`、`log`、`chrono` 和 `zstd` 等“明确不采用（直接依赖）”仍可能是已批准后端的合法传递依赖；Cargo 统一版本并由 cargo-deny 审计，但 hyastro 源码不直接依赖。NOVAS 的唯一状态是“仅开发/验证”，不进入生产依赖。

## 附录 A：本地路径引用索引（ref/）

| 引用点 | 路径 |
|---|---|
| anise workspace 元数据/版本/许可 | ref/anise/Cargo.toml |
| anise features 定义 | ref/anise/anise/Cargo.toml:71-77 |
| anise 模块入口 | ref/anise/anise/src/lib.rs:15-33 |
| anise 唯一 unsafe | ref/anise/anise/src/lib.rs:92 |
| Almanac 结构（线程安全） | ref/anise/anise/src/almanac/mod.rs:66-82 |
| SPK 求值分派（类型 1-13/其余 Unsupported） | ref/anise/anise/src/ephemerides/translate_to_parent.rs:59-120 |
| DataType 枚举（1-21） | ref/anise/anise/src/naif/daf/data_types.rs:29-44 |
| Aberration 9 模式 | ref/anise/anise/src/astro/aberration.rs:53-95 |
| 光行时迭代（1 vs 3 次） | ref/anise/anise/src/ephemerides/translations.rs:142-179 |
| MetaAlmanac 下载/CRC32/HTTP 端点 | ref/anise/anise/src/almanac/metaload/metaalmanac.rs |
| embed_ephem 构建期下载 | ref/anise/anise/build.rs |
| 差分验证规模/容差 | ref/anise/anise/README.md:172；ref/anise/anise/tests/orientations/validation.rs:33-40 |
| fuzz 目标 | ref/anise/anise/fuzz/fuzz_targets/（25 个） |
| hifitime no_std/features/闰秒语义 | ref/hifitime/src/lib.rs:3；ref/hifitime/Cargo.toml；ref/hifitime/README.md:249,274-277,296 |
| sofars 版本/许可/SOFA 条款 | ref/sofars/Cargo.toml:2-6；ref/sofars/LICENSE |
| rsofa/erfa-sys/novas/nyx/rust-astro 元数据 | ref/rsofa/Cargo.toml、ref/erfa-sys/erfa-sys/Cargo.toml、ref/novas/Cargo.toml、ref/nyx-space/Cargo.toml、ref/rust-astro/Cargo.toml |

## 附录 B：crates.io 元数据快照（2026-08-05 查询）

`crate|max_stable_version|license|rust-version`（节选）：

```
thiserror|2.0.19|MIT OR Apache-2.0|1.71        serde|1.0.229|MIT OR Apache-2.0|1.56
bitflags|2.13.1|MIT OR Apache-2.0|1.56        tracing|0.1.44|MIT|1.65
tracing-log|0.2.0|-|-                          nalgebra|0.35.0|Apache-2.0|1.89
uom|0.38.0|Apache-2.0 OR MIT|1.68             num-traits|0.2.19|MIT OR Apache-2.0|1.60
libm|0.2.16|MIT|1.63                          hifitime|4.3.0|MPL-2.0|-
sofars|0.6.1|MIT|-                             anise|0.10.4|MPL-2.0|-
rayon|1.12.0|MIT OR Apache-2.0|1.80           sha2|0.11.0|MIT OR Apache-2.0|1.85
memmap2|0.9.11|MIT OR Apache-2.0|1.65         winnow|1.0.4|MIT|1.65
csv|1.4.0|Unlicense/MIT|1.73                  fitsrs|0.4.1|Apache-2.0 OR MIT|-
fitsio|0.21.10|MIT/Apache-2.0|1.58            votable|0.7.0|Apache-2.0 OR MIT|-
cdshealpix|0.9.1|Apache-2.0 OR MIT|1.81       moc|0.19.2|Apache-2.0 OR MIT|-
arrow|59.2.0|Apache-2.0|1.85                  parquet|59.2.0|Apache-2.0|1.85
geographiclib-rs|0.2.7|MIT|1.70               sgp4|2.4.0|MIT|-
jiff|0.2.35|Unlicense OR MIT|1.70             flate2|1.1.9|MIT OR Apache-2.0|1.67
miniz_oxide|0.9.1|MIT OR Zlib OR Apache-2.0|- zstd|0.13.3|MIT|1.64
bzip2|0.6.1|MIT OR Apache-2.0|1.82            argmin|0.11.0|-|-
roots|0.0.8|-|-                               rand|0.10.2|MIT OR Apache-2.0|1.85
rand_chacha|0.10.0|MIT OR Apache-2.0|1.85     rand_distr|0.6.0|MIT OR Apache-2.0|1.85
approx|0.5.1|Apache-2.0|-                     proptest|1.11.0|MIT OR Apache-2.0|1.85
trybuild|1.0.120|MIT OR Apache-2.0|1.88       rstest|0.26.1|MIT OR Apache-2.0|1.70
criterion|0.8.2|Apache-2.0 OR MIT|1.86        iai-callgrind|0.16.1|Apache-2.0 OR MIT|1.74
arbitrary|1.4.2|MIT OR Apache-2.0|1.63        tempfile|3.27.0|MIT OR Apache-2.0|1.63
indexmap|2.14.0|Apache-2.0 OR MIT|1.85        zerocopy|0.8.55|BSD-2-Clause OR Apache-2.0 OR MIT|1.56
bytes|1.12.1|MIT|1.57                         der|0.8.1|Apache-2.0 OR MIT|1.85
crc32fast|1.5.0|MIT OR Apache-2.0|1.63        ureq|3.3.0|MIT OR Apache-2.0|1.85
url|2.5.8|MIT OR Apache-2.0|1.63              regex|1.13.1|MIT OR Apache-2.0|1.65
serde-lexpr|0.1.3|MIT OR Apache-2.0|-          hyperdual|1.5.0|MIT|-
snafu|0.9.2|MIT OR Apache-2.0|1.65            cargo-deny|0.20.2|MIT OR Apache-2.0|1.88
cargo-audit|0.22.2|Apache-2.0 OR MIT|1.88     cargo-semver-checks|0.50.0|Apache-2.0 OR MIT|1.93
cargo-msrv|0.19.3|Apache-2.0 OR MIT|1.91      vsop87|3.0.0|Apache-2.0|-
rust-spice|0.7.8|-|-                          celestial-eop-data|0.1.x|MIT OR Apache-2.0|-
cargo-fuzz|0.13.2|-|-                         celestial|0.1.0|-|-
```

（`-` = 该字段在 crates.io API 返回中缺失或为 null；`brentroot` 与 `cds-votable-rust`（crate 名）查询无结果——前者疑已下架，后者发布名实为 `votable`。全部条目以 crates.io API 实测为准，已在本文件相应节引用。）

## 附录 C：遗留约束核对

- 本轮未修改 `ref/` 任一仓库，也未运行 cargo 构建、测试、lint 或格式化；配套 `docs/` 已同步 ANISE 和依赖决策。
- 所有"确定/可选"条目均给出版本、feature、默认、no_std、unsafe/FFI、许可、MSRV、平台、不自己实现理由、替代项；"可选"条目全部有启用条件与关闭语义。
- 无占位符、无待定结论：`brentroot` 等异常项已给明确结论（不采用/自研）；CDS FITS 依赖已核对为已发布的 `fitsrs =0.4.1`，并禁止误用名称相近的 `fitrs`。
