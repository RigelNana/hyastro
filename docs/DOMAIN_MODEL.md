# hyastro 领域与错误模型

- 文档状态：设计基线
- 适用范围：P0 公开领域类型、模块职责、上下文和错误语义
- 配套词汇：`CONTEXT.md`

## 1. 设计约束

hyastro 的类型系统负责阻止单位、时间尺度、参考系、原点、历元和结果阶段的误用。静态且有限的语义进入类型参数或受约束包装；外部数据、具体参考架实现和动态 SPICE 帧作为受检值存在。

设计遵循以下不变量：

1. 公开接口不使用裸 `f64` 表达带单位、范围或领域语义的量。
2. 常用静态参考系统、原点和时间尺度在编译期区分；动态身份在进入静态路径前完成校验。
3. 点、自由向量、方向、位置、速度和状态具有不同类型及合法运算集合。
4. 位置阶段由不同类型表达，无法对同一结果重复应用折射、光行差等修正。
5. 时变数据和模型选择来自不可变上下文，计算不读取隐藏全局状态。
6. 上游库类型停留在适配器实现内，hyastro 公开类型是唯一领域事实来源。
7. 纯数学和时间表示保持 `no_std` 兼容；单值热路径不分配。
8. 一个真实实现使用具体类型；第二个真实实现出现后才稳定适配器接缝。

## 2. 领域模块

| 模块 | 拥有的概念 | 主要依赖 | 公开深模块 |
| --- | --- | --- | --- |
| `math` | 量、角语义、向量、点、方向、矩阵、旋转、球面几何、数值算法 | 无 | 值类型及其方法 |
| `time` | 历法、日期、瞬间、时长、时间尺度、JD/MJD、闰秒、EOP 时间量 | `math` | `TimeContext` |
| `frame` | 参考系统、参考架、原点、旋转、状态变换 | `math`、`time` | `Frames` |
| `earth` | 旋转椭球、地理坐标、站点、地球定向链 | `math`、`time`、`frame` | `Earth` |
| `ephem` | 天体、状态查询、覆盖、内核清单和段选择 | `math`、`time`、`frame` | `Ephemeris` |
| `astro` | 星表位置、空间运动、天体测量位置、视位置和观测位置 | `time`、`frame`、`earth`、`ephem` | `Astrometry` |
| `event` | 根、极值、接触、状态转换和可观测窗口 | `time`、`astro`、`ephem` | `Events` |
| `catalog` | 星表记录、字段约定、协方差和流式解析 | `math`、`time`、`frame` | 具体格式读取器 |
| `media` | 折射、对流层、电离层和传播介质结果 | `math`、`earth` | 具体模型及 `Atmosphere` |

共享基础值类型放在拥有其不变量的领域模块中，再由需要它们的深模块重导出。首个实现阶段不创建通用 `utils`、全局 `context` 或根级后端 trait。

依赖方向保持单向：

```text
math -> time -> frame -> earth
                 |        |
                 v        v
ephem ----------> astro -> event
catalog --------> astro <- media
```

## 3. 核心类型模型

### 3.1 量与角语义

规范存储固定为 `f64`：角为弧度、长度为米、速度为米每秒、时长为整数纳秒级表示或等价固定精度表示。单位只出现在命名构造器和读取方法中。

基础量使用透明包装：

```rust
/// 任意有限有符号角，规范存储单位为弧度
#[repr(transparent)]
pub struct Angle(f64);

/// 有限长度，规范存储单位为米
#[repr(transparent)]
pub struct Length(f64);

/// 有限速度，规范存储单位为米每秒
#[repr(transparent)]
pub struct Speed(f64);
```

角的领域语义使用受约束包装：`RightAscension`、`Declination`、`Longitude`、`Latitude`、`HourAngle`、`Azimuth`、`Altitude`、`ZenithDistance`、`PositionAngle`、`PhaseAngle` 和 `Separation`。构造器分为两类：

- `try_*` 验证范围并保留输入含义，例如 `Declination::try_deg(91.0)` 返回范围错误。
- `wrap_*` 执行该语义允许的规范化，例如 `Longitude::wrap_deg(361.0)` 得到 `1°`。

同维量可以显式转换单位。不同语义量不提供隐式 `From`，需要命名转换，例如 `Altitude::zenith_distance()`。

### 3.2 空间、时间和变换

常用静态语义使用封闭标记类型。标记 trait 由 hyastro 密封，外部代码无法伪造一个具有未知轴定义的参考系或时间尺度。

```rust
/// 在参考系 F 中表达、分量物理量为 Q 的自由向量
pub struct Vector3<F, Q> {
    components: [f64; 3],
    marker: PhantomData<fn() -> (F, Q)>,
}

/// 在参考系 F、原点 O 下表达的三维位置点
pub struct Point3<F, O> {
    meters: [f64; 3],
    marker: PhantomData<fn() -> (F, O)>,
}

/// 参考系 F 中经验证的单位方向
pub struct Direction<F> {
    unit: Vector3<F, Dimensionless>,
}

/// 同一参考系、原点和历元下的位置速度状态
pub struct State<F, O, S> {
    position: Point3<F, O>,
    velocity: Vector3<F, Speed>,
    epoch: Instant<S>,
}

/// 从 From 到 To 的主动或被动约定已固定的旋转
pub struct Rotation<From, To> {
    matrix: Matrix3,
    marker: PhantomData<fn(From) -> To>,
}

/// 包含旋转、平移和时间导数的六维状态变换
pub struct StateTransform<From, To> {
    rotation: Rotation<From, To>,
    angular_velocity: Vector3<To, AngularSpeed>,
    translation: Vector3<To, Length>,
    translation_rate: Vector3<To, Speed>,
}
```

`Point3 - Point3` 仅产生同一 `F`、`O` 下的位移；`Point3 + Vector3<_, Length>` 产生点；两个点不相加。`Direction` 只能经有限且非零向量构造。`Rotation<A, B>` 只与 `Rotation<B, C>` 复合。

时间核心类型如下：

- `Date<C>`：历法 `C` 中的年月日。
- `DateTime<C, S>`：历法 `C` 和时间尺度 `S` 下的日期时间标签。
- `Instant<S>`：物理时间线上的瞬间，以尺度 `S` 表示。
- `Duration`：两个瞬间之间的物理间隔，不携带时间尺度。
- `JulianDate<S>`、`ModifiedJulianDate<S>`：保留双分量的连续日表示。
- `Epoch<S>`：供坐标、星表或轨道参数引用的参考瞬间。

时间尺度转换只由 `TimeContext::convert` 完成。`Instant<Utc>` 与 `Instant<Tt>` 没有直接 `From` 实现。UTC 日期时间标签允许合法的 `23:59:60`，其构造需要闰秒数据覆盖。

常用静态参考系统使用 `Icrs`、`Bcrs`、`Gcrs`、`Cirs`、`Tirs` 和 `Itrs` 标记。实际支持某个具体 ICRF/ITRF 实现时，为它定义具体标记类型。动态 SPICE 帧保持为适配器内的受检 `DynamicFrame`，只提供运行时检查的变换方法；确认轴、原点和时间语义完全匹配后才能转换到静态类型。

球面坐标按语义分开：

- `SphericalPosition<F>`：经度、纬度和距离。
- `EquatorialPosition<F>`：赤经和赤纬，仅适用于赤道类参考系。
- `HorizontalPosition`：方位和高度，绑定站心与方位约定。
- `Direction<F>`：无距离方向，作为球面算法的规范输入。

### 3.3 位置阶段与结果

天体测量修正链使用不同结果类型：

```text
CatalogPlace<F>
    -> AstrometricPlace<F>
    -> ApparentPlace<F>
    -> VacuumObservedPlace<Horizontal>
    -> ObservedPlace<Horizontal>
```

每个转换方法只接受合法的前置阶段，阶段类型直接防止重复修正。逆向计算使用独立命名结果，避免伪装成无损逆变换。

高层结果只保存完成后续计算所需的主值和强类型语义，包括参考系、原点、时间尺度、历元及是否含折射。数值迭代结果可以包含残差、次数和最终括区间。

## 4. 上下文与工作流

上下文是构造完成后不可变、可安全共享的算法输入。上下文不联网、不读取环境变量、不自动选择 latest 数据，也不依赖进程级可变状态。

- `TimeContext` 拥有闰秒表、EOP 数据、插值策略和时间尺度模型，提供转换及差值查询。
- `Frames` 拥有岁差章动、地球定向数据、动态参考架注册和变换路径，提供方向、位置与状态变换。
- `Earth` 拥有椭球、站点和地球定向工作流，提供地理坐标及站点状态。
- `Ephemeris` 拥有冻结顺序的内核清单和查询能力，提供经过覆盖检查的状态。
- `Astrometry` 组合时间、参考系、历表、观测者、引力体和大气策略，提供位置阶段转换与完整观测工作流。
- `Events` 组合判据、覆盖预检、扫描、括根和精化，返回完整且排序稳定的事件集合。

调用者学习高层任务接口即可完成标准路径：

```rust
/// 使用显式时间数据把 UTC 瞬间转换到 TT
let tt: Instant<Tt> = time.convert(utc)?;

/// 在同一上下文中计算指定站点的观测位置
let observed = astrometry.observe(&catalog_place, tt, &site)?;

/// 在闭区间内搜索完整的升落事件集合
let events = events.rise_set(&target, interval, &site)?;
```

底层标准算法保留在模块内部或专家子模块。SOFA、hifitime 和 ANISE 的调用顺序及数据布局不进入公开接口。

## 5. 错误模型

### 5.1 错误原则

hyastro 不提供根级万能 `hyastro::Error`。每个深模块拥有一个 `#[non_exhaustive]` 错误枚举，高层工作流通过带 `source` 的变体保留下层错误。调用者可以只处理当前任务真实存在的失败模式。

错误表示无法按请求语义产生结果。调用者显式选择的近似模型或外推策略属于请求的一部分；模型适用范围之外仍然返回错误。

以下结果属于成功语义：

- 事件区间内没有根或极值，返回空事件集合。
- 极昼、极夜、拱极和永不升起，返回明确分类。
- 调用者显式选择近似模型或允许外推，且请求仍处于该策略声明的适用范围。

以下情况必须返回错误：

- 非有限输入、范围错误或非法组合。
- 请求落在数据覆盖之外且未选择外推策略。
- 缺少完成计算所需的闰秒、EOP、历表或气象数据。
- 后端不支持目标、参考系、段类型或请求能力。
- 几何量在数学上未定义。
- 已括区间的数值精化未收敛。
- 外部格式损坏、越界或超过资源限制。
- 请求超出所选模型或外推策略的适用范围。

### 5.2 模块错误

| 错误类型 | 稳定变体族 | 必须携带的信息 |
| --- | --- | --- |
| `math::Error` | `NonFinite`、`OutOfRange`、`Degenerate`、`InvalidRotation`、`NoBracket`、`NonConvergent` | 字段、值与范围；退化分类；残差、次数和最终区间 |
| `time::Error` | `InvalidDate`、`NonexistentTime`、`AmbiguousTime`、`MissingData`、`Coverage`、`UnsupportedScale` | 历法/尺度、原标签、所需数据、请求与可用覆盖 |
| `frame::Error` | `FrameMismatch`、`OriginMismatch`、`PathNotFound`、`MissingEop`、`UnsupportedFrame` | 源/目标参考系、原点、历元和模型 |
| `earth::Error` | `InvalidEllipsoid`、`UndefinedGeodeticPosition`、`SiteMismatch`、`Frame` | 椭球、站点、位置和下层来源 |
| `ephem::Error` | `UnknownTarget`、`UnknownCenter`、`UnsupportedFrame`、`UnsupportedSegment`、`Coverage`、`CenterCycle`、`CorruptKernel`、`Backend` | 查询、内核、段、覆盖和底层原因 |
| `astro::Error` | `IncompleteCatalogData`、`InvalidMotion`、`Time`、`Frame`、`Ephemeris`、`Atmosphere`、`NonConvergent` | 目标、观测者、时刻、修正阶段和下层来源 |
| `event::Error` | `InvalidInterval`、`Coverage`、`Evaluation`、`BudgetExceeded`、`Cancelled`、`NonConvergent` | 判据、区间、求值次数、最后括区间和下层来源 |
| 格式适配器错误 | `Malformed`、`UnsupportedVersion`、`InvalidField`、`OutOfBounds`、`ResourceLimit`、`Io` | 文件偏移、记录/行/列、字段、限制和来源文件 |

共享的叶错误值保持具体语义：

- `RangeError` 存储字段、带单位值和允许区间。
- `CoverageError` 存储数据种类、带尺度请求区间和可用区间。
- `ConvergenceError` 存储算法、物理量残差、迭代次数和最终括区间。
- `BackendFailure` 存储操作种类，并在 `std` 构建中保留类型擦除的错误源。
- `ParseLocation` 存储字节偏移以及可用的记录、行或列位置。


适配器必须把上游错误映射到所属模块的稳定语义。公开枚举不包含 ANISE、hifitime、SOFA、I/O 库或解析库的类型。普通坏输入不得触发 panic。核心 `no_std` 错误优先保持无分配；文件和后端错误只存在于 `std` 模块。

## 6. 验收场景

以下场景用于验证领域与错误模型是否落地：

1. `Declination::try_deg(91.0)` 返回带度单位范围的 `math::Error::OutOfRange`。
2. 从零向量构造 `Direction<Gcrs>` 返回 `DegenerateGeometry::ZeroNorm`。
3. 对跖方向的角距离成功返回 `π`；重合方向的位置角返回明确的未定义几何错误。
4. `State<Gcrs, EarthCenter, Tt>` 无法传给要求 `State<Itrs, EarthCenter, Ut1>` 的接口，编译失败。
5. 动态 SPICE 帧定义与请求静态帧不一致时返回 `frame::Error::FrameMismatch`。
6. UTC 到 UT1 转换缺少 EOP 时返回 `time::Error::MissingData`；覆盖外查询返回独立的 `Coverage` 变体。
7. 历表存在目标但不支持段类型时返回 `UnsupportedSegment`；目标不存在时返回 `UnknownTarget`。
8. 折射输入超出所选模型的适用高度时返回 `media::Error::OutOfDomain`。
9. 升落搜索遇到拱极目标时返回成功分类；求值预算耗尽时返回 `event::Error::BudgetExceeded`。
10. 数值求根失败返回残差、迭代次数和最后括区间，不返回零值或空结果。
11. 高层观测错误保留下层错误源，调用者无需解析错误字符串。
12. 公开类型、错误和序列化结果均不出现上游 crate 类型。
