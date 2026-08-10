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
| `frame` | 参考系统、参考架、轴向约定、语义方向坐标、原点、状态与变换 | `math`、`time` | `Frames` |
| `earth` | 旋转椭球、地理坐标、站点、地球定向链 | `math`、`time`、`frame` | `Earth` |
| `ephem` | 天体、状态查询、覆盖、内核清单和段选择 | `math`、`time`、`frame` | `Ephemeris` |
| `astro` | 接收光行时、星表位置、空间运动、天体测量位置、视位置和观测位置 | `time`、`frame`、`earth`、`ephem` | `Astrometry` |
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

`RightAscension` 与 `HourAngle` 都使用 `[0, 2π)`，也就是 `[0h, 24h)` 的规范区间。`HoursMinutesSeconds` 表达该无符号周期区间；`DegreesMinutesSeconds` 使用独立的 `SexagesimalSign` 保存正负号，因此能区分正零与负零。两者只负责六十进制表示和文本往返，不承担民用时间语义。

同维量可以显式转换单位。不同语义量不提供隐式 `From`，需要命名转换，例如 `Altitude::zenith_distance()`。

`RootOptions` 拥有括根求解的显式横坐标容差、函数残差容差和迭代预算；`bisect` 提供确定性二分，`brent` 在始终保留符号括区间的前提下采用逆二次插值、割线或二分回退。调用方不得把大基准绝对时刻直接压入 `f64`，时间事件以括区间起点的相对秒数精化。

### 3.2 空间、时间和变换

常用静态语义使用封闭标记类型。天文参考系统和时间尺度的标记 trait 由 hyastro 密封；纯 `math` 值仍可携带调用者自有的幻影标签，但只有受检的 `frame` 标记能进入标准框架变换。

计算坐标框架 `F` 绑定参考系统或具体参考架、原点、轴定义、手性和历元/分点元数据。原点是 `F` 的关联语义，不作为可与 `F` 任意组合的第二个类型参数。

```rust
/// 在计算坐标框架 F 中表达、分量物理量为 Q 的自由向量
pub struct Vector3<F, Q> {
    components: [Q; 3],
    marker: PhantomData<F>,
}

/// 在计算坐标框架 F 下表达的三维位置点；F 已绑定原点
pub struct Point3<F> {
    coordinates: Vector3<F, Length>,
}

/// 计算坐标框架 F 中经验证的单位方向
pub struct Direction<F> {
    unit: Vector3<F, Dimensionless>,
}

/// 同一计算坐标框架和历元下的位置速度状态
pub struct State<F, S> {
    position: Point3<F>,
    velocity: Vector3<F, Speed>,
    epoch: Instant<S>,
}

/// 把源坐标分量变换为目标坐标分量的旋转
pub struct Rotation<From, To> {
    matrix: Matrix3,
    marker: PhantomData<fn(From) -> To>,
}

/// 在一个物理历元有效、包含旋转、平移和时间导数的状态变换
pub struct StateTransform<From, To, S> {
    epoch: Instant<S>,
    rotation: Rotation<From, To>,
    angular_velocity: Vector3<To, AngularSpeed>,
    translation: Vector3<To, Length>,
    translation_rate: Vector3<To, Speed>,
}
```

`Point3 - Point3` 仅产生同一 `F` 下的位移；`Point3 + Vector3<_, Length>` 产生点；两个点不相加。`Direction` 只能经有限且非零向量构造。`Rotation<A, B>` 只与 `Rotation<B, C>` 复合，且只直接作用于自由向量和方向；点和状态必须通过含原点语义的 `StateTransform`。

状态变换统一采用源分量到目标分量的被动坐标变换：

```text
r_to = R_from_to r_from + t
v_to = R_from_to v_from + ω × (R_from_to r_from) + t_dot
```

`t` 是源原点相对目标原点的位置，以目标框架表达；`t_dot` 是该坐标的物理秒导数。`ω` 以目标框架表达，并定义为满足 `R_dot Rᵀ = [ω]×` 的轴向量。变换组合顺序为 `A→B` 后接 `B→C`。状态变换携带有效物理历元，应用和组合时必须核对历元值。

时间核心类型如下：

- `Date<C>`：历法 `C` 中的年月日。
- `DateTime<C, S>`：历法 `C` 和时间尺度 `S` 下的日期时间标签。
- `Instant<S>`：物理时间线上的瞬间，以尺度 `S` 表示。
- `Duration`：两个瞬间之间的物理间隔，不携带时间尺度。
- `JulianDate<S>`、`ModifiedJulianDate<S>`：保留双分量的连续日表示。
- `Epoch<S>`：供坐标、星表或轨道参数引用的参考瞬间。
- `TimeInterval<S>`：同一尺度下严格有序的非空闭物理时间区间，端点均包含在内。
- `FixedUtcOffset`：短于 24 小时的恒定有符号 UTC 秒差，不携带时区规则。
- `CivilDateTime<C>`：历法日期、常规日内时刻和 `FixedUtcOffset` 的组合；UTC 闰秒瞬间因移位后的 `:60` 不满足常规标签不变量而明确不可表示。
- `LeapSeconds<'a>`：无分配、版本化的闰秒数据，显式保存起始偏移、覆盖范围和过期日；`LeapSecond` 只表示真正的 ±1 秒事件。
- `EarthOrientationRecord`：IERS 解析后的强类型记录，保留 UTC/MJD 历元、计算分量和原始空列。`EarthOrientationData` 保存同一产品的有序记录；`try_earth_rotation_samples_in` 只提取 UT1 域，`try_earth_attitude_samples_in` 提取方向旋转所需的 UT1、极移和天极偏差，`try_samples_in` 转换完整 EOP 状态样本。三条路径都要求显式覆盖区间和 `EarthOrientationAcceptance`；观测/预报来源在模块内部按极移、UT1、LOD、天极四域执行，拒绝项不会被静默跳过，源误差列在没有协方差传播模型前不进入公共接口。
- `EarthRotationSample`、`EarthRotationTable<'a>`：仅保存和插值 `UT1−UTC` 的窄能力路径；表不可变、版本化、带覆盖和过期边界，跨闰秒插值连续的 `UT1−TAI`。它足以支持 UT1、ERA 和恒星时，不伪造或要求无关的 LOD、极移与天极偏差，也不能驱动地固系状态变换。
- `EarthAttitudeSample`、`EarthAttitudeTable<'a>`：方向姿态能力，包含 `UT1−UTC`、`xp`、`yp`、`dX`、`dY`，但不要求 LOD。它可生成绑定历元的 `EarthAttitudeSolution<S>` 并执行观测 GCRS/CIRS/TIRS/ITRS 方向旋转；不能声称具有地固系状态变换所需的角速度。
- `EarthOrientationSample`：可进入完整地球姿态热路径的 EOP 样本，包含某个 UTC 标记物理瞬间的 `UT1−UTC`、LOD、`xp`、`yp`、`dX`、`dY` 强类型值，以及可选的极移变化率。原始空列不能转换为零；缺少算法必需量时转换明确失败。
- `EarthOrientationTable<'a>`：不可变、版本化、带覆盖和过期边界的完整 EOP 数据；只在首末样本闭区间内插值，不外推；跨闰秒先插值连续的 `UT1−TAI`。存在相邻样本时，表可由样本差分推导极移与天极偏差变化率。
- `SiderealTimeSolution<S>`：同一 `Instant<S>` 的 ERA、GMST、GAST、equation of origins/equinoxes 与 TT/UT1 快照；地方平/视恒星时由显式东经计算。该结果只要求 `EarthRotationTable`，姿态和完整 EOP 表也可提供同一能力。
- `EarthAttitudeSolution<S>`、`EarthOrientationSolution<S>`：同一 `Instant<S>` 的不可变观测地球姿态快照；两者均保存一致求值的 TT、UT1、EOP 方向量、IAU 2006/2000A CIP/CIO 模型和带历元的 `GCRS → CIRS → TIRS → ITRS` 旋转。完整 `EarthOrientationSolution` 另保存 LOD/变化率并提供状态变换；应用 `dX/dY` 后重新计算 operational CIO locator，CIO/equinox 等价矩阵明确只比较未加观测修正的模型链。
- `DeltaT<S>`：绑定物理历元的 `TT−UT1` 查询结果。观测路径由同一次 EOP `UT1−UTC`、适用的 `TAI−UTC` 和精确定义 `TT−TAI = 32.184 s` 组合，不跨闰秒产生伪跳变；预测路径由 `DeltaTModel` 在 TT 闭有效区间内直接求值，因此不依赖未知的未来 UTC 或闰秒。
- `DeltaTModel<'a>`、`PredictedEarthOrientation<'a>`：前者是具名、带闭有效区间和 `PredictionDisposition` 的 `TT−UT1` 模型，可选逐次返回标准不确定度；内置 NASA/Espenak–Meeus 2006 分段多项式、显式常量场景和调用者函数接缝互不混用。后者把一个 `DeltaTModel` 与具名 `EarthAttitudeOffsetModel` 组合为方向姿态能力；极移和天极偏差可以是预测值或明确零假设，LOD、`UT1−UTC`、未来 UTC 及其闰秒均不被伪造。`EarthAttitudeState<S>` 和 `EarthAttitudeModelProvenance` 使表测值与预测/假设值沿同一只读求值接缝进入框架和站心天测，同时保留来源、处置和可用不确定度。
- `GeocentricTdb`、`TdbSolution<S>`：显式选择的地心 TDB 解析模型及其不可变结果；结果同时保存同一历元的 TT、TDB 和 `TDB−TT`，不暗示站心项或历表积分。

尺度转换由目标类型发起：`Instant::<S>::from_instant(source, &model)` 证明模型覆盖后保留精确内部 TAI 坐标，`JulianDate::<S>::from_instant(source, &model)` 计算目标尺度数值。`TimeScaleModel<S>` 是密封能力 trait；普通 `TimeContext<NoEarthOrientation>` 只实现 UTC/TAI/TT/GPS，加入 `EarthRotationTable`、`EarthAttitudeTable` 或 `EarthOrientationTable` 后的上下文实现 UT1 并公开观测驱动的 `delta_t_at`。方向姿态只要求 `EarthAttitudeTable`，地固系状态变换和固定站点质心观测者要求包含 LOD/变化率的完整 `EarthOrientationTable`。`GeocentricTdb` 以显式模型身份实现地心 TDB，hifitime adapter 实现其支持的模型尺度；因此 TDB 不被伪装成固定偏移，也不由普通 `TimeContext` 隐式选择。不存在无条件公开重标或直接跨尺度 `From`。UTC 日期时间标签允许合法的 `23:59:60`；`TimeContext::new` 接受显式 `LeapSeconds`，`TimeContext::builtin` 使用 IERS Bulletin C 72 快照。

常用静态计算坐标框架使用 `Icrs`、`Bcrs`、`Gcrs`、`Cirs`、`Tirs` 和 `Itrs` 标记；每个标记关联唯一原点和元数据。实际支持某个具体 ICRF/ITRF 实现时，为它定义单独的具体标记类型。动态 SPICE 帧保持为适配器内的受检 `DynamicFrame`，只提供运行时检查的变换方法；确认轴、原点和时间语义完全匹配后才能转换到静态类型。
赤道和黄道方向由密封的 `EquatorialAxes`、`EclipticAxes` 能力约束。`MeanEquatorEquinoxOfDate`、`TrueEquatorEquinoxOfDate`、`MeanEclipticEquinoxJ2000`、`MeanEclipticEquinoxOfDate`、`TrueEclipticEquinoxOfDate` 与 `Galactic` 只定义方向轴；没有空间原点，因此不能进入 `Point3`、`State` 或 `StateTransform`。`TrueEclipticEquinoxOfDate` 明确采用 Astropy/ERFA 惯例：IAU 2006 frame bias/岁差与 IAU 2000A 章动得到日期真赤道/真分点，再以真黄赤交角 $\epsilon_A+\Delta\epsilon$ 转到日期真黄道；该旋转不表示已经应用任何视位置修正。日期相关角坐标由带 `Instant<S>` 的结果包装，不能脱离求值历元作为完整结果传播。


球面方向坐标按语义分开：

- `SphericalDirection<F>`：通用经度、纬度方向几何，不声称赤道、黄道或银道语义。
- `EquatorialDirection<F>`：赤经、赤纬，`F` 必须实现 `EquatorialAxes`。
- `EclipticDirection<F>`：黄经、黄纬，`F` 必须实现 `EclipticAxes`；黄经/黄纬不能与银经/银纬混用。
- `GalacticDirection`：IAU 1958 银道系统的 Hipparcos ICRS 规范实现。
- `EquatorialDirectionAt<F, S>`、`EclipticDirectionAt<F, S>`：日期相关方向及其物理求值历元。
- `Direction<F>`：无距离方向，作为球面算法和轴旋转的规范笛卡尔输入。
- `SphericalPosition<F>`：未来的经度、纬度和距离位置；必须使用带明确空间原点的完整计算坐标框架。
- `HorizontalDirection`：北起向东增加的可选方位角和闭区间高度/天顶距；天顶、天底方位无定义时为 `None`。它只表示局部方向值，站点、历元和折射阶段由包含它的高层结果承担。

大地坐标按物理定义分开：

- `ReferenceEllipsoid`：由模型标识、长半轴和扁率定义的旋转参考面；WGS 84 与 GRS 80 是不同值。
- `GeodeticPosition`：`GeodeticLongitude`、`GeodeticLatitude` 与有符号 `EllipsoidalHeight` 的组合。
- `GeocentricLatitude`：ITRS 位置向量相对赤道面的纬度；地心原点明确无定义。
- `FixedSite`：在 ITRS 中位置与速度固定的站点，不暗示具体 ITRF 实现或站点运动模型。
- `EastNorthUp<F>`、`NorthEastDown<F>`：在类型化参考架 `F` 中表达的局部切向基；转换到 GCRS 时绑定求值历元并应用完整 EOP 链。
- `TopocentricFrame<S>`：一个固定站点在物理历元 `Instant<S>` 的运行时站心参考架快照，保留站点 GCRS 位置/速度及同历元 ENU 基；它不是可脱离站点和历元复用的静态 frame marker。

### 3.3 位置阶段与结果

天体测量修正链使用不同结果类型：

```text
InfiniteCatalogPlace
    -> AstrometricCatalogPlace<S>
    -> VacuumObservedCatalogPlace<S>
    -> ObservedCatalogPlace<S>

SpatialCatalogPlace <-> BarycentricCatalogState
SpatialCatalogPlace
    -> AstrometricSpatialCatalogPlace<S>
    -> VacuumObservedSpatialCatalogPlace<S>
    -> ObservedSpatialCatalogPlace<S>

finite solar-system body
    -> ReceptionLightTime<Bcrs, S>
    -> VacuumObservedPlace<S>
         |-> VacuumApparentDisk<S>
         `-> ObservedPlace<S>

geocentric Sun
    -> SolarApparentPlace<S>
    -> SolarTimeSolution<S>
```

每个转换方法只接受合法的前置阶段，阶段类型直接防止重复修正。逆向计算使用独立命名结果，避免伪装成无损逆变换。

高层结果只保存完成后续计算所需的主值和强类型语义，包括参考系、原点、时间尺度、历元及是否含折射。数值迭代结果可以包含残差、次数和最终括区间。

- `StandardUncertainty<Q>`：绑定原物理量 `Q` 的有限非负一倍标准不确定度，并保留该量的规范单位；它只表达结果拥有的单项误差证据，不暗示独立、Gaussian、完整协方差、系统误差或模型差异已经建模。
- `CorrelationMatrix<N>`：有限、对称、半正定且对角线为一的无量纲相关矩阵。它只表达系数；参数顺序和单位由拥有它的领域结果定义。
- `SpatialCatalogCovariance`：有限距离六参数星表解的完整协方差，固定顺序为 $\alpha*$、$\delta$、$\varpi$、$\mu_{\alpha *}$、$\mu_\delta$、$v_r$；前三项使用 rad，自行使用 rad/s，径向速度使用 m/s。$\alpha*$ 只表示当前历元局部切平面微分 $d\alpha\cos\delta$，不是可跨天球使用的全局坐标。
- `SpatialCatalogPlaceWithCovariance`：一个物理 `SpatialCatalogPlace` 及其同历元协方差。历元传播围绕同一 SOFA `starpm` 模型在输入和输出局部切平面求五点数值 Jacobian，以 $J C J^\mathsf{T}$ 传播并保留 Jacobian 作为数值证据；天极的 $\alpha*$ 基奇异性必须显式拒绝。
- `EarthAttitudeStandardUncertainties`：地球姿态结果上逐字段可缺失的 IERS 源标准不确定度。C04 最终值保留源记录误差；`finals2000A` 仅对实际采用的 Bulletin A 值保留 A 误差，采用 Bulletin B 时不错误绑定 A 误差。区间内采用相关性未知的线性上界并以 `UncertaintyOrigin` 标识，任一端缺值则结果继续缺失。

- `ReceptionLightTime<Bcrs, S>`：目标在发射历元、观测者在接收历元求值后的自由相对位置；保存双历元、自然视线方向、距离、单程光行时、迭代次数和时间残差，不提供跨历元相减得到的伪速度。
- `FixedObserverAt<S>`：一个 `FixedSite` 在单一接收历元的可复用天测上下文，冻结同一次 `EarthAttitudeState<S>` 求值、`TopocentricFrame<S>`、地球历表状态、站点质心位置/速度及 SOFA 星无关光行差参数。完整 `EarthOrientationTable` 路径使用观测 LOD/帧率；`PredictedEarthOrientation` 或其他无 LOD 姿态路径使用明确标记的 IERS 名义自转率。同站点同历元的多个有限目标可复用该值。
- `ParallaxMeasurement`：星表拟合得到的有符号周年视差及非负标准不确定度；零或负中心值仍是有效测量，但 `try_physical` 必须失败，不能隐式产生距离。
- `Parallax`：严格为正且有限的物理周年视差；与 `ParallaxMeasurement` 分离，使噪声测量和可用于空间运动的距离参数不会混淆。
- `CatalogRadialVelocity`：太阳系质心处的天体测量径向速度，正值表示退行；不把光学、射电或相对论光谱速度定义隐式混用。
- `InfiniteCatalogPlace`：一个明确受限的无限远 ICRS 星表位置，保存 TCB 参考历元、赤经/赤纬及 `CatalogProperMotion`。后者固定采用 $\mu_{\alpha *}=\dot{\alpha}\cos\delta$ 与 365.25 日 TCB 儒略年；该输入以类型表达零视差和零径向速度。
- `SpatialCatalogPlace`：物理有限距离的六参数 ICRS 星表位置，组合 TCB 参考历元、赤经/赤纬、`CatalogProperMotion`、正 `Parallax` 与 `CatalogRadialVelocity`。`propagate_to` 使用 SOFA `starpm` 联合传播视差、自行和径向速度，包含直线空间运动、变化光行时造成的透视效应及 Stumpff 特殊相对论调整；SOFA 的距离替换、超速清零或不收敛回退均被显式拒绝。
- `BarycentricCatalogState`：与 ICRS 对齐、以 SSB 为原点并绑定 TCB 历元的非零亚光速三维位置速度。它通过 SOFA `starpv` / `pvstar` 与 `SpatialCatalogPlace` 双向转换，也可按恒定惯性速度传播；它不是太阳系天体的 `RelativeState`。
- `ApparentMagnitude<B, Z>`：同时以类型参数绑定光度通带 `B` 和星等系统 `Z` 的有限视星等；负值合法，不同通带或零点约定不能直接求差。`MagnitudeDifference` 表示有向星等差，`FluxRatio` 表示严格正的同带通量比，并按 Pogson 关系双向换算。Johnson V 与 Vega、AB、ST 只定义光度语义，不伪造未实现的绝对通量标定。
- `AstrometricCatalogPlace<S, C>`：使用 SOFA `pmpx` 在 SSB 处把由 `C` 保留的星表语义传播到观测时刻的 ICRS 方向，并保存 TCB 观测历元及经过的儒略年。默认 `C = InfiniteCatalogPlace`；`AstrometricSpatialCatalogPlace<S>` 是 `C = SpatialCatalogPlace` 的别名。非零 $\mu_{\alpha *}$ 在天极无法转换为坐标赤经率时显式报错。
- `VacuumObservedCatalogPlace<S, C>`：由同历元 `FixedObserverAt<S>` 接受相应 `AstrometricCatalogPlace<S, C>` 后产生。链依次应用观测者相关 Roemer 项、有限源的周年及周日视差、SOFA `ldsun` 太阳远源偏折、组合站点速度光行差、IAU 2006/2000A BPN、地球自转、极移和真空地平投影，并分别保存 Roemer、视差、偏折和光行差角修正。空间源别名为 `VacuumObservedSpatialCatalogPlace<S>`；该星表链不伪造太阳系目标式发射历元或迭代光行时。
- `ObservedCatalogPlace<S, C>`：消耗相应真空星表结果并显式应用同一 SOFA 大气折射模型；没有再次应用折射的方法。空间源别名为 `ObservedSpatialCatalogPlace<S>`。
- `VacuumObservedPlace<S>`：有限太阳系目标的站心真空观测结果，保存接收/发射双历元、CIRS 中间赤道方向、局部 `HorizontalDirection`、距离、光行时、有限距离太阳单极偏折诊断及收敛诊断。当前链应用站心视差、太阳不透明盘面判断、太阳单极偏折、由地球公转与站点运动共同产生的相对论光行差、IAU 2006/2000A 地球姿态和极移；明确不包含大气折射或 Shapiro 延迟。
- `SphericalBodyFigure`：一个物理天体、正半径与版本化模型标识的不可拆分组合。内置形状只提供 IAU 2015 名义太阳球和 IAU WGCCRE 2015 月球参考球；系统质心不能拥有物理表面。
- `VacuumApparentDisk<S>`：由 `VacuumObservedPlace<S>` 和同目标 `SphericalBodyFigure` 派生的圆形真空视盘，使用收敛距离和精确 `asin(R/Δ)` 视半径。它保留中心、模型与全视直径；同站点同历元视盘可比较中心角距、带符号边缘间隙和重叠拓扑，但不伪装成含大气差分折射的盘面。
- `HorizonCriterion`：升落搜索的受检判据，把中心参考高度、`Vacuum`/`Refracted(AtmosphericConditions)` 坐标阶段和 `HorizonDiskPoint::{Center, UpperLimb, LowerLimb}` 绑定为一个值。球形盘面边缘在每次求值时按 `SphericalBodyFigure` 与收敛站心距离动态计算视半径；折射盘面判据只在中心应用所选折射模型，再把真空球形视半径作为垂直偏移，不伪造大气差分折射造成的盘面压缩。
- `ObservedPlace<S>`：由 `VacuumObservedPlace<S>::apply_refraction` 消耗真空阶段后产生，保存来源真空结果、折射后 `HorizontalDirection`、调用者显式提供的 `AtmosphericConditions`，以及带 SOFA 模型适用范围分类的 `RefractionCorrection`。该结果类型不公开再次应用折射的方法，编译期阻止重复修正。
- `AtmosphericConditions`：一次观测的不可变气象与波段输入，由强类型气压、摄氏温度、相对湿度和微米波长组成；不属于永久站点身份。零气压显式表示真空，不存在隐式标准大气。
- `GeocentricApparentPlace<S>`：有限太阳系目标相对地心的接收视位置。地球固定在接收历元，目标迭代到发射历元；结果应用光线近太阳历元的有限源太阳单极偏折、地球质心周年光行差和 IAU 2006/2000A 方向链，并同时保留目标身份、完整 `ReceptionLightTime<S>`、GCRS、`TrueEquatorEquinoxOfDate`、`TrueEclipticEquinoxOfDate`、太阳偏折处置和收敛诊断。它不包含站心视差、周日光行差、折射或 Shapiro 延迟。
- `SolarApparentPlace<S>`：`GeocentricApparentPlace<S>` 的太阳专用视图；保留相同方向、双历元、距离和诊断，并明确记录太阳不会由自身点质量模型发生自偏折。
- `MeanSolarTime`：由 UT1 日内分数和东正经度定义的名义 24 小时钟面读数；不是 UTC、时区或民用日期时间。
- `ApparentSolarTime`：真太阳当地时角加 12 小时所得的名义 24 小时钟面读数；不是匀速时间尺度。
- `EquationOfTime`：同一经度的 `ApparentSolarTime − MeanSolarTime`，以 `(-12h, 12h]` 内的有符号 `Duration` 保存；正值表示真太阳领先，经度不改变该差值。
- `SolarTimeSolution<S>`：同一物理历元的 `SolarApparentPlace<S>`、`SiderealTimeSolution<S>`、格林尼治平/真太阳时和时差的不可变组合；地方太阳时只应用同一个东正经度偏移，不引入 UTC offset 或时区规则。
- `EventEvidence<S>`：事件精化后的物理时间括区间、半宽时刻不确定度、有符号判据残差、迭代次数和求值次数。
- `ExtremumEvidence<S>`：有界 Brent 极值精化后的最终时间括区间、半宽时刻不确定度、迭代次数和累计天测求值次数；物理目标值由具体事件结果保存，不把角度、长度或坐标压成同一裸标量结果。
- `SolarEclipseModel`：太阳与月球的两个受检 `SphericalBodyFigure`，共同定义地方日食的圆形视边缘；默认值使用 IAU 2015 名义太阳球和 IAU WGCCRE 2015 月球球，调用者可显式替换模型以复现采用不同太阳半径的权威资料。`SolarEclipseSearchOptions` 将该模型与既有 `AngularEventSearchOptions` 绑定，不把半径约定藏在算法实现中。
- `LocalSolarEclipseObservation<S>`：同一固定站点、同一接收历元的太阳/月球真空视盘组合，保留中心角距、盘面拓扑、强类型食分、遮掩面积比例、月心位置角和太阳水平坐标。食分是沿盘心连线被月球覆盖的太阳直径比例，可在全食时大于一；`SolarObscuration` 是被遮太阳视盘面积比例，范围为 `[0,1]`，两者不能互换。
- `SolarEclipseContact<S>`：地方日食 C1/C2/C3/C4 的一个外切或内切根，保留完整视盘观测、太阳边缘接触位置角和 `EventEvidence<S>`。`LocalSolarEclipseMaximum<S>` 则保存直接最大化食分所得的观测与 `ExtremumEvidence<S>`；它不是 C1/C4 时刻的中点。
- `LocalSolarEclipse<S>`：按食甚是否落入调用者闭区间归属的一次固定站点偏食、环食或全食。结果总是求全 C1 至 C4；环食/全食另有 C2/C3，并保留阶段时长、站点、球形边缘模型、EOP 版本与 `EphemerisProvenance`。结果是无折射几何，`solar_disk_is_above_horizon` 只回答真空太阳盘是否至少部分高于天文地平线，不把天气或标准大气伪装成可见性。
- `BesselianFundamentalPlane<S>`：通过地心且垂直于月球阴影轴的历元绑定基本平面；保留由月球指向太阳、位于日期真赤道与真春分点轴上的 `Z` 方向，并规定 `+x` 向东、`+y` 向北。平面不保存裸矩阵，也不把日期轴方向误写成静态 J2000 方向。
- `BesselianPlaneCoordinate` 与 `BesselianShadowRadius`：分别保存以所选地球赤道半径为单位的 `x/y` 和 `l1/l2`。`l2 < 0` 是出版物中的本影约定，与 `SolarShadowRadius > 0` 表示本影的物理影锥约定方向相反，两个类型不得互换。
- `BesselianLimbModel`：贝塞尔影锥采用的显式太阳半径、月球半径和月球位置修正约定。`physical` 从一个 `SolarEclipseModel` 派生相同的半影/本影月球球面与零修正；`nasa_five_millennium` 明示 NASA 目录采用的 `k1=0.272488`、`k2=0.272281`、696000 km 太阳半径和零 `Δb/Δl`。调用者构造的非零 `Δb/Δl` 分别加到月球日期真黄纬/真黄经，并在阴影轴计算前转回日期真赤道方向。它与全球分类使用的物理 `SolarEclipseModel` 是不同领域概念，不允许隐式切换。
- `BesselianElements<S>`：一个物理历元的 `x,y,d,μ,l1,l2,tan f1,tan f2` 及 `BesselianElementDerivatives`。结果保留基本平面、地球、`BesselianLimbModel`、历表 provenance 和天测求值数。即时入口使用 60 秒 TT 对称导数模板；多项式求值使用解析导数，两者由 `BesselianDerivativeMethod` 区分。`μ` 是以 TT 为自变量的历书时角；把根数应用于旋转地球时必须显式另给 `ΔT=TT−UT1`。
- `BesselianElementsPolynomial<S>`：以一个物理参考历元为 `t=0`、仅在显式闭区间内有效的短期根数模型。五个等间隔视位置样本按 NASA 六小时表方法拟合 `x/y` 三次、`d/l1/l2` 二次和 `μ` 一次多项式；内部另保留阴影轴赤经三次拟合以重建完整基本平面。结果保存各项采样最大残差、常量 `tan f1/f2`、地球、半径模型、历表 provenance 和拟合天测求值数；区间外求值拒绝外推。
- `SolarEclipseGamma`：全球食甚时月球阴影轴到地心的有符号距离，以所选参考椭球的赤道半径为单位；阴影轴最近点位于日期真赤道以北为正、以南为负。它不是食分，也不单独决定偏食、全食或环食。
- `SolarShadowRadius`：阴影轴法平面上的有符号核心影锥半径；正值表示锥顶前的本影，负值表示锥顶后的伪本影，零表示锥顶。其绝对值才是对应截面的几何半径。`GlobalSolarEclipseMaximum<S>` 的 `geometric_core_shadow_radius_*` 来自分类所选 `SolarEclipseModel` 的物理公切锥；`BesselianElements<S>::contact_core_shadow_radius_at_fundamental_plane` 则严格等于所选 `BesselianLimbModel` 的 `-l2*a`，两个模型结果不得只标成无来源的 `core`。
- `GlobalSolarEclipseMaximum<S>`：直接最小化阴影轴地心距离所得的全球食甚切片，保留 `SolarEclipseGamma`、轴距、精确公切半影/本影/伪本影锥与参考椭球的交会判定、物理公切锥在轴法平面和中心轴近侧地表的带符号半径，以及 `ExtremumEvidence<S>`。
- `GlobalSolarEclipseCentralPath<S>`：阴影轴与参考椭球相交的完整物理时间区间，起止都是轴线与椭球相切的数值根；另保留路径刚进入/食甚/即将离开时的环食或全食性质，以及锥顶穿过近侧地表时精化得到的 `HybridSolarEclipseTransition<S>`。它只描述中心轴的时间切片，不伪装成地理中心线或南北界。
- `SolarEclipseCentralPhase<S>`：一个固定中心线站点上的 C2、C3 和精确纳秒舍入的 `C3−C2`。两个接触以同一贝塞尔核心影残差在固定 ITRS 地表点求根，不把全球食甚或路径采样步长当作地方接触。
- `GlobalSolarEclipsePathPoint<S>`：一个物理时刻的地理中心线截面，保存中心线地理位置、运动核心影包络的北/南界、两界之间的参考椭球反解测地跨度 `boundary_geodesic_span`、按贝塞尔投影公式得到的横向 `path_width`、中心阶段、环食/全食性质和无折射太阳水平坐标。路径界必须同时满足核心影锥落在地表以及固定地表点接触残差的时间导数为零；它不是瞬时影斑的最大/最小纬度，边界测地跨度也不是路径横向宽度。
- `GlobalSolarEclipsePath<S>`：由 `GlobalSolarEclipse<S>`、同历表同椭球的六小时 `BesselianElementsPolynomial<S>`、参考历元的 `DeltaT<S>` 和显式 `GlobalSolarEclipsePathOptions` 组成的地理路径。结果保留完整阴影轴相交时间区间、双边包络都存在时的有序采样、半径模型、地球、历表与 `TT−UT1` 来源；日出/日落附近只有单边包络的时刻不伪造为完整截面。`Delta T` 在短期多项式窗口内保持为调用者给定值，不能由 EOP 静默猜测。
- `GlobalSolarEclipse<S>`：按全球食甚是否落入调用者闭区间归属的一次偏食、环食、全食或全环食。核心影锥可在轴线不穿过参考椭球时形成非中心全食或环食；这类结果显式没有 `GlobalSolarEclipseCentralPath<S>`，不能与偏食或普通中心食混同。结果保留地球椭球、球形日月模型和 `EphemerisProvenance`，不隐式要求 UT1/EOP；地理路径由显式 `Events::solar_eclipse_path` 工作流另行计算。
- `RelativeBodyQuery`：两个不同天体、明确的“目标减参考”次序及 `Geometric`/`Apparent` 求值模式的受检组合。地心或固定站点不是该组合的隐式默认值，而由所调用的 `Events` 工作流选择并在结果的 `ObservationOrigin` 中保留。
- `ConfigurationEvent<S>`：一个合、冲或东方/西方照事件，保留判据所用的日期真黄经差或日期真赤经差、两天体的完整 `EventBodyPosition<S>`、观测原点和 `EventEvidence<S>`。太阳参考合事件可由同一观测原点到目标和太阳的实际距离进一步分类为内合或外合。
- `GreatestElongationEvent<S>`：两个天体真实球面角距的局部极大值，保留目标位于参考天体东侧/西侧的分支、两天体位置和 `ExtremumEvidence<S>`。`StationEvent<S>` 则保留日期真黄经率过零前后的运动方向、角速度残差和单独的 `StationEvidence<S>`。
- `AngularSeparationExtremumEvent<S>`、`DistanceExtremumEvent<S>`、`CoordinateExtremumEvent<S>` 与 `CoordinateCrossingEvent<S>`：分别表达真实球面角距极值、同时几何天体间距离极值、日期真黄纬/赤纬极值和这些坐标的升/降交越；不同物理量不共用裸 `f64` 事件结果。
- `SolarTermEvent<S>`：太阳地心视黄经到达一个规定 $15^\circ$ 网格位置的事件，保留完整 `SolarApparentPlace<S>` 与事件证据。
- `SolarTermYear`：由固定 UTC 偏移定义公历年归属并按当地民用时间排序的 24 个 `SolarTermYearEntry`；每项同时保存 UTC 物理事件和固定偏移公历标签。

## 4. 上下文与工作流

上下文是构造完成后不可变、可安全共享的算法输入。上下文不联网、不读取环境变量、不自动选择 latest 数据，也不依赖进程级可变状态。

- `TimeContext<'a, E>` 拥有闰秒策略，并用类型参数 `E` 表达地球自转或姿态能力；`with_earth_rotation` 接收只含 `UT1−UTC` 的 `EarthRotationTable`，`with_earth_attitude` 接收方向旋转所需的 `EarthAttitudeTable`，`with_earth_orientation` 接收完整 `EarthOrientationTable`，`with_predicted_earth_orientation` 接收从 TT 直接求 UT1 的显式预测场景。`resolve_fixed`/`represent_fixed` 在同一闰秒策略下转换 `CivilDateTime`，不引入时区数据库；预测场景不扩大 UTC 覆盖，也不生成未来闰秒。所有输入数据都必须已经验证、不可变且版本化。
- `Frames` 借用一个具备相应数据能力的 `TimeContext`。任意时间上下文都可通过 `celestial_orientation_at` 只用 TT 生成带历元的 `CelestialOrientationSolution`；任一 UT1 上下文都可通过 `sidereal_time_at` 生成 `SiderealTimeSolution`；姿态上下文通过 `earth_attitude_at` 执行方向链；只有完整 EOP 上下文公开带速度的 `earth_orientation_at`、`at` 和 `transform`。观测地球定向主路径是 `GCRS → CIRS → TIRS → ITRS`：IAU 2006/2000A CIP/CIO 模型加入 EOP `dX/dY`，ERA 使用 UT1，极移使用 `xp/yp` 与 TIO locator `s′`；状态变换还要求 LOD 与可用的 EOP 变化率，方向旋转不伪造这些导数。
- `Earth` 绑定一个显式 `ReferenceEllipsoid`，提供测地坐标与 `Point3<Itrs>` 的双向转换、地心纬度、固定站点和 ITRS ENU/NED 基。`topocentric_frame_at` 由完整 EOP 状态变换产生 `TopocentricFrame<S>`；`topocentric_frame_with_nominal_rotation_at` 保留姿态表的 UT1、极移和天极偏差，并显式以 IERS 名义角速度产生站点惯性速度。结果携带 `SiteVelocityModel`，不把缺失 LOD 伪造成观测值。地心原点、无效椭球和空站点标识均明确失败。
- `EphemerisProvider` 是天体测量层唯一依赖的历表接缝：输入 `EphemerisQuery<Bcrs, S>`，返回未施加光行时、偏折、光行差或大气修正的 `RelativeState<Bcrs, S>`，并提供连续 `Coverage` 与 `EphemerisProvenance`。`EphemerisQuery<F, S>` 同时固定目标、中心、物理历元和参考轴；相对状态以自由位置/速度向量表达目标相对中心，不把动态中心伪装成 `Point3<Bcrs>` 的固定 SSB 原点。默认 `SofaAnalyticEphemeris` 是无文件、有限目标和有限精度的解析后端；`anise` feature 下的 `Ephemeris` 是显式本地 `KernelManifest` 驱动的高精度 SPK 后端。两者不自动回退或混合。
- `Astrometry<'context, 'data, E, P>` 显式借用 `TimeContext` 与实现 `EphemerisProvider` 的 `P`；`Events` 保留同一提供者类型。`geocentric_apparent_place(target, epoch, options)` 对任意受所选后端支持的有限太阳系目标执行“地球接收状态 → 目标发射时刻迭代 → 地心自然视线 → 光线近太阳历元与有限源太阳单极偏折/遮挡判断 → 地球质心周年光行差 → GCRS/日期真赤道/日期真黄道”链；`solar_apparent_place` 是同一实现的太阳专用视图，`lunar_illumination_at` 则组合月—地、日—地和日—月三条收敛光行时，在月球发射事件上计算物理相位角与球形月面照亮比例，同时保留地球接收历元的日月视角距、有向视黄经差和盈亏分支。具备 UT1 的上下文还可由 `solar_time` 将太阳视位置与 GAST 组合为平/真太阳时及时差。完整 EOP 上下文的 `fixed_observer_at(&site, epoch)` 使用观测 LOD/帧率；`PredictedEarthOrientation` 上同名入口使用场景姿态与名义自转率；其他不含 LOD 的姿态上下文必须显式调用 `fixed_observer_with_nominal_rotation_at`。结果的 `SiteVelocityModel` 始终区分观测与名义速率。两条路径都返回可批量复用的 `FixedObserverAt<S>`；它分别以 `vacuum_observed_catalog_place`、`vacuum_observed_spatial_catalog_place` 和 `vacuum_observed_place` 接受无限远星表源、六参数空间星表源和有限太阳系目标。
- `HorizonsCompatibleLunarV`：只消费已完成的 `LunarIllumination<S>`，以日—月照明腿距离、月—地接收腿距离和月球处物理相位角计算地心、无大气、假定未受地影衰减的积分月面 Johnson V/Vega 星等。`GeocentricLunarVMagnitude<S>` 保留原始照明几何、距离与相位星等项、模型标识和 `LunarVApplicability`；后者显式区分正常区、相位角小于 $7^\circ$ 的已知模型偏差区，以及月面与地球本影/半影相交的月食区。月食区返回值只是未受食时的模型基线，不代表实际月食亮度。
- `Events` 以 `Astrometry` 为唯一真实计算上下文。`solar_terms_in` 连续化太阳视黄经并搜索 $15^\circ$ 网格；`moon_phase_angle_in` 连续化月球减太阳的视黄经差并搜索调用者给出的强类型 `MoonPhaseAngle`，`moon_phases_in` 则在一次扫描中搜索 $0^\circ/90^\circ/180^\circ/270^\circ$ 四个目标，并把同一 `MoonPhaseAngleEvent` 内核包装为命名主相位。`configurations_in` 搜索合、冲和方照，`greatest_elongations_in` 搜索角距极大，`stations_in` 搜索视赤经率过零；角距、物理距离、日期真黄纬/赤纬的极值和交越由各自命名工作流承担。除同时几何天体间距离外，这些事件均有地心入口；完整 EOP 上下文另提供固定站点入口并复用 `FixedObserverAt` 的站心视位置链。根事件共享 `AngularEventSearchOptions`、`EventEvidence` 和内部括根/Brent 精化，极值事件共享 `ExtremumSearchOptions`、`ExtremumEvidence` 和内部有界 Brent 精化，但每项保留自己的领域查询、结果类型与物理量。`solar_term_year` 和 `moon_phase_year` 再以显式固定 UTC 偏移筛选公历年份。模块不公开任意谓词框架，避免调用者重建视位置链或丢失时间尺度、模型和数值证据。
- `MeasuredCycle<K, S>` 只由相邻同类物理事件构成，保留强类型周期种类、首尾 `CycleBoundary<S>`、实际 `Duration`、搜索证据、参考轴和所选后端的 `EphemerisProvenance`；ANISE 后端的 provenance 进一步保留冻结的 `KernelManifest`，解析后端保留稳定模型标识。`CycleStatistics<K>` 只接收这些完整周期。`Events` 分别公开分点年、恒星年、近点年、交点年及五种月球月周期工作流：固定 J2000 黄道用于恒星周期，日期平均黄道用于回归月与交点事件，径向速度负到正的根定义近地点/近日点。`ModeledCycle<TropicalYear, S>` 与事件测量类型分离，保留求值历元、模型标识和适用范围。
- `Events::local_solar_eclipses_in` 在完整 `EarthOrientationTable` 或显式 `PredictedEarthOrientation` 上公开：先以地心视朔为每个朔望月的候选种子，再在种子附近对固定站点视盘食分做有界极值精化；无重叠的朔被丢弃，外切和内切残差分别用括根 Brent 精化。局部食分最大而不是最小盘心角距定义食甚，因为两个视半径随时间变化。表测路径保留 EOP 与观测 LOD；预测路径从 `DeltaTModel` 取得 UT1，以具名偏移模型生成姿态，并显式采用名义地球自转率。接触可越过请求区间边界以保持一次食的序列完整；历表、EOP 或预测模型覆盖不足必须失败，不以截断序列、`UT1=UTC` 或伪造未来闰秒回退。
- `Events::global_lunar_eclipses_in` 以地心视满月为候选，在日期真赤道轴上求月心到反日地影轴的距离极小，并以 `LunarEclipseModel` 中的球形日月和 `LunarShadowConvention` 计算影半径。`LunarShadowConvention` 把 Danjon 的有效地球视差修正与 Chauvenet 的完成影半径缩放分成两个系数；模型标签、系数、历表来源和数值证据随 `GlobalLunarEclipse<S>` 保留。P1/P4、U1/U4、U2/U3 分别是半影外切、本影外切和本影内切根；半影/偏食/全食区间彼此嵌套，不能合并为一个无类型持续时间。`local_lunar_eclipse_visibility` 只消费已完成的全球结果和完整 EOP，在固定站点求月出月落交点并裁剪各嵌套阶段；接触样本同时保留月球高度、低空标志、太阳高度和曙暮光背景。它不重新定义全球接触，也不把天气、地形地平线或月食亮度模型混入几何可见性。

调用者学习高层任务接口即可完成标准路径：

```rust
/// 目标尺度类型通过显式模型转换同一物理瞬间
let tt = Instant::<Tt>::from_instant(utc, &time)?;
let ut1 = JulianDate::<Ut1>::from_instant(utc, &time_with_eop)?;

/// 同一完整 EOP 上下文生成保留历元尺度的框架状态
let tirs: State<Tirs, Utc> = Frames::new(&time_with_eop).transform(cirs)?;

/// 一个站点和接收历元准备一次，可计算多个有限太阳系目标
let observer = astrometry.fixed_observer_at(&site, utc)?;
let sun = observer.vacuum_observed_place(CelestialBody::Sun, options)?;
let horizontal = sun.horizontal();
let disk = sun.apparent_disk(SphericalBodyFigure::IAU_2015_NOMINAL_SUN)?;
let angular_diameter = disk.diameter();
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
| `ephem::Error` | `UnknownTarget`、`UnknownCenter`、`UnsupportedFrame`、`UnsupportedSegment`、`Coverage`、`CenterCycle`、`CorruptKernel`、`KernelIo`、`Backend` | 查询、内核、段、覆盖和底层原因 |
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
4. `State<Gcrs, Tt>` 无法传给要求 `State<Itrs, Tt>` 的接口，编译失败；框架变换保留历元的表示尺度，变换对象与状态的物理历元不同时返回 `frame::Error::EpochMismatch`。
5. 动态 SPICE 帧定义与请求静态帧不一致时返回 `frame::Error::FrameMismatch`。
6. `Instant::<Ut1>::from_instant(utc, &TimeContext<NoEarthOrientation>)` 因缺少 `TimeScaleModel<Ut1>` 而编译失败；EOP 覆盖外和过期查询分别返回 `EarthOrientationUnavailable` 与 `EarthOrientationExpired`。
7. 历表存在目标但不支持段类型时返回 `UnsupportedSegment`；目标不存在时返回 `UnknownTarget`。
8. 折射输入超出所选模型的适用高度时返回 `media::Error::OutOfDomain`。
9. 升落搜索遇到拱极目标时返回成功分类；求值预算耗尽时返回 `event::Error::BudgetExceeded`。
10. 数值求根失败返回残差、迭代次数和最后括区间，不返回零值或空结果。
11. 高层观测错误保留下层错误源，调用者无需解析错误字符串。
12. 公开类型、错误和序列化结果均不出现上游 crate 类型。
