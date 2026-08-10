# hyastro 完整功能点目录

本文档是实现范围的原子功能目录，是 `docs/PRD.md` 的展开版。PRD 说明产品目标、优先级和验收；本文档用于架构拆分、里程碑规划、issue 建立和覆盖核验。

## 1. 标记

- **P0**：可信基础，没有它不能形成正确的上层接口。
- **P1**：首个完整用户版本需要的常用工作流。
- **P2**：高级、专门或数据量较大的扩展。
- **内核**：不依赖文件、网络或全局状态的纯计算。
- **数据**：需要版本化外部数据。
- **适配**：连接上游库、格式或后端。
- **工作流**：组合多个模型的高层功能。

每个条目是一项可独立验证的能力。低优先级条目仍须使用真实实现，不能以空实现进入稳定接口。

## 2. 基础量与单位

### 2.1 角度

- **F-ANG-001 P0 内核** 从弧度构造和读取角度。
- **F-ANG-002 P0 内核** 从十进制度构造和读取角度。
- **F-ANG-003 P0 内核** 从度、角分、角秒构造和格式化。
- **F-ANG-004 P0 内核** 从时、分、秒构造和格式化时角。
- **F-ANG-005 P0 内核** 支持角分、角秒、毫角秒、微角秒和纳角秒换算。
- **F-ANG-006 P0 内核** 支持圈、弧度、度和时之间的精确比例换算。
- **F-ANG-007 P0 内核** 规范化到 `[0, 2π)`。
- **F-ANG-008 P0 内核** 规范化到 `(-π, π]`。
- **F-ANG-009 P0 内核** 计算周期角的最短有符号差与无符号角距。
- **F-ANG-010 P0 内核** 连续角序列解包裹与重新包裹。
- **F-ANG-011 P0 内核** 赤经类型，范围为一整圈。
- **F-ANG-012 P0 内核** 赤纬和纬度类型，范围为 `[-π/2, π/2]`。
- **F-ANG-013 P0 内核** 经度类型及东经/西经显式构造。
- **F-ANG-014 P0 内核** 时角类型，规范范围为 `[0, 2π)`，即 `[0h, 24h)`。
- **F-ANG-015 P0 内核** 方位角类型，默认北起向东增加。
- **F-ANG-016 P0 内核** 高度和天顶距互转。
- **F-ANG-017 P0 内核** 相位角、位置角和距角语义类型。
- **F-ANG-018 P0 内核** 角速度与每秒、每日、每儒略年单位。
- **F-ANG-019 P1 内核** DMS/HMS 宽松输入和严格规范输出。
- **F-ANG-020 P1 内核** 负零、进位、舍入位数和 Unicode 度分秒符号处理。
- **F-ANG-021 P1 内核** 六十进制字符串与十进制字符串无损往返策略。
- **F-ANG-022 P0 内核** `sin`、`cos`、`tan` 接收角，反三角函数返回角。
- **F-ANG-023 P0 内核** 稳定的 `atan2` 方位角和象限处理。

### 2.2 其他物理量

- **F-QTY-001 P0 内核** 长度：米、公里、天文单位、光秒、光年、秒差距。
- **F-QTY-002 P0 内核** 速度：米每秒、公里每秒、天文单位每日。
- **F-QTY-003 P0 内核** 加速度与角加速度。
- **F-QTY-004 P0 内核** 时长：秒、毫秒、微秒、纳秒、日、儒略年。
- **F-QTY-005 P0 内核** 质量与标准引力参数 `GM` 分开建模。
- **F-QTY-006 P0 内核** 绝对温度与摄氏输入。
- **F-QTY-007 P1 内核** 气压：Pa、hPa、mbar。
- **F-QTY-008 P1 内核** 相对湿度、绝对湿度和水汽压。
- **F-QTY-009 P1 内核** 波长、频率和波数。
- **F-QTY-010 P1 内核** 光程、时间延迟和群延迟。
- **F-QTY-011 P1 内核** 星等、通量和无量纲比例。
- **F-QTY-012 P0 内核** 带单位绝对/相对容差。
- **F-QTY-013 P1 内核** 测量值加标准差的轻量表示。
- **F-QTY-014 P1 内核** 非对称误差和置信区间表示。

当前 `ApparentMagnitude<B, Z>` 以类型参数同时绑定光度通带和星等系统，支持有限负星等、同语义星等差以及与严格正 `FluxRatio` 的双向换算；内置 Johnson V 通带和 Vega、AB、ST 零点标记。物理绝对通量及其带单位谱密度仍未实现，因此 F-QTY-011 只覆盖星等与无量纲比例部分。

## 3. 线性代数与旋转

### 3.1 三维向量、点和状态

- **F-VEC-001 P0 内核** 固定大小三维向量，无堆分配。
- **F-VEC-002 P0 内核** 加、减、标量乘除、点积、叉积和模。
- **F-VEC-003 P0 内核** 归一化和零向量错误。
- **F-VEC-004 P0 内核** 两向量夹角的稳定计算。
- **F-VEC-005 P0 内核** 投影、拒绝分量和正交基。
- **F-VEC-006 P0 内核** 区分自由向量和有原点的点。
- **F-VEC-007 P0 内核** 区分位置向量、速度向量和加速度向量。
- **F-VEC-008 P0 内核** 位置加自由位移和两点作差。
- **F-VEC-009 P0 内核** 同一参考系、原点和历元下的位置速度状态。
- **F-VEC-010 P0 内核** 状态平移、相对状态和中心变换。
- **F-VEC-011 P0 内核** 有限值检查和分量级诊断。
- **F-VEC-012 P1 内核** 切向、径向和法向分解。
- **F-VEC-013 P1 内核** 批量切片、迭代器和调用者输出缓冲。
- **F-VEC-014 P1 适配** 与 `nalgebra` 固定向量的显式互转。
- **F-VEC-015 P2 适配** 与 ndarray/Arrow 列式批量数据互转。

### 3.2 矩阵

- **F-MAT-001 P0 内核** 固定 `3×3` 矩阵存储和索引。
- **F-MAT-002 P0 内核** 矩阵乘法、矩阵向量乘法和转置。
- **F-MAT-003 P0 内核** 行列式和一般逆的受检实现。
- **F-MAT-004 P0 内核** 旋转矩阵的正交性、有限性和行列式检查。
- **F-MAT-005 P0 内核** 绕 X/Y/Z 轴的主动/被动旋转约定。
- **F-MAT-006 P0 内核** 从源参考系到目标参考系的类型化旋转。
- **F-MAT-007 P0 内核** 旋转复合、逆和恒等旋转。
- **F-MAT-008 P0 内核** `6×6` 状态变换或等价块结构。
- **F-MAT-009 P1 内核** 近似旋转矩阵正交化并返回残差。
- **F-MAT-010 P1 内核** 矩阵导数、角速度反对称矩阵和状态速度项。
- **F-MAT-011 P1 内核** Jacobian 与协方差 `J C Jᵀ` 传播。

### 3.3 四元数和姿态

- **F-QUAT-001 P0 内核** 单位四元数构造、验证和规范化。
- **F-QUAT-002 P0 内核** 共轭、逆、乘法和旋转向量。
- **F-QUAT-003 P0 内核** 四元数与旋转矩阵互转。
- **F-QUAT-004 P0 内核** 轴角与四元数互转。
- **F-QUAT-005 P1 内核** 指定序列的内禀/外禀欧拉角互转。
- **F-QUAT-006 P1 内核** SLERP、短弧选择和端点稳定性。
- **F-QUAT-007 P1 内核** 姿态加角速度和时间传播接口。
- **F-QUAT-008 P2 适配** SPICE CK 姿态样本和角速度适配。
- **F-QUAT-009 P2 内核** 姿态插值质量和覆盖检查。

## 4. 球面坐标与球面算法

### 4.1 坐标原语

- **F-SPH-001 P0 内核** 经度、纬度和可选距离的球面坐标。
- **F-SPH-002 P0 内核** 赤经、赤纬天空方向。
- **F-SPH-003 P0 内核** 球面方向与单位笛卡尔向量互转。
- **F-SPH-004 P0 内核** 带距离球面坐标与笛卡尔位置互转。
- **F-SPH-005 P0 内核** 局部东、北、径向正交基。
- **F-SPH-006 P0 内核** 极点处经度不定和零距离错误。

### 4.2 球面几何

- **F-SPH-010 P0 内核** 大圆角距离，稳定处理极小距离和对跖点。
- **F-SPH-011 P0 内核** 两点初始/终止位置角。
- **F-SPH-012 P0 内核** 已知起点、方位和角距求终点。
- **F-SPH-013 P0 内核** 球面三角形 SSS、SAS、ASA/AAS 求解。
- **F-SPH-014 P0 内核** 球面三角形面积和球面超量。
- **F-SPH-015 P1 内核** 两大圆交点和退化分类。
- **F-SPH-016 P1 内核** 点到大圆和小圆的最近距离。
- **F-SPH-017 P1 内核** 点到球面弧段最近点。
- **F-SPH-018 P1 内核** 球面线性插值。
- **F-SPH-019 P1 内核** 单位方向的加权均值和离散度。
- **F-SPH-020 P1 内核** 球冠/圆锥包含与相交。
- **F-SPH-021 P1 内核** 经度跨零的球面矩形。
- **F-SPH-022 P2 内核** 球面多边形包含、面积、交和并。
- **F-SPH-023 P2 内核** 大圆轨迹穿越和最近接近。
- **F-SPH-024 P1 内核** gnomonic、stereographic、orthographic 和 zenithal 投影原语。

## 5. 常数与数值工具

### 5.1 常数

- **F-CON-001 P0 数据** π、τ、角度比例和日秒数。
- **F-CON-002 P0 数据** SI 定义常数和真空光速。
- **F-CON-003 P0 数据** IAU 天文单位和秒差距换算。
- **F-CON-004 P0 数据** IAU 名义太阳、地球、木星半径、质量参数和光度。
- **F-CON-005 P0 数据** SOFA/ERFA 使用的标准纪元和时间比例常数。
- **F-CON-006 P0 数据** WGS84、GRS80 和可选历史地球椭球参数。
- **F-CON-007 P1 数据** 行星半径、扁率、GM 和质量比的版本化数据集。
- **F-CON-008 P1 数据** 标准天体类型与适配器内部编号的映射。
- **F-CON-009 P1 数据** 每个常数的单位、标准版本和精确性说明。
- **F-CON-010 P0 数据** 不允许把 EOP、闰秒或历表状态固化成无版本常数。

### 5.2 数值算法

- **F-NUM-001 P0 内核** Horner 多项式及导数。
- **F-NUM-002 P0 内核** Chebyshev 多项式、Clenshaw 求值及导数。
- **F-NUM-003 P0 内核** Kahan/Neumaier 稳定求和。
- **F-NUM-004 P0 内核** 线性插值和有单位输入。
- **F-NUM-005 P0 内核** Hermite 插值。
- **F-NUM-006 P1 内核** Lagrange 等间隔/不等间隔插值。
- **F-NUM-007 P1 内核** 单调三次与自然三次样条。
- **F-NUM-008 P0 内核** 二分求根。
- **F-NUM-009 P0 内核** Brent 有括区间求根。
- **F-NUM-010 P1 内核** Newton/割线与括区间保护混合求根。
- **F-NUM-011 P0 内核** 一维有界极小/极大搜索。
- **F-NUM-012 P0 内核** 周期函数扫描、括根和根去重。
- **F-NUM-013 P1 内核** 切触根和重复根识别。
- **F-NUM-014 P1 内核** 自适应 Simpson/Gauss-Kronrod 积分，用于大气或曝光积分。
- **F-NUM-015 P1 内核** 有单位有限差分和数值 Jacobian。
- **F-NUM-016 P1 内核** 对称矩阵、Cholesky、半正定检查和最近半正定修复策略。
- **F-NUM-017 P2 内核** 蒙特卡洛采样和确定性随机源注入。
- **F-NUM-018 P0 内核** 所有迭代返回状态、残差、次数和最终区间。

## 6. 历法、日期与时间表示

### 6.1 历法

- **F-CAL-001 P0 内核** 推算公历日期验证和闰年。
- **F-CAL-002 P0 内核** 推算儒略历日期验证和闰年。
- **F-CAL-003 P0 内核** 显式改革日的混合 Julian/Gregorian 历法。
- **F-CAL-004 P0 内核** 天文年编号，包括第 0 年和负年份。
- **F-CAL-005 P1 内核** BCE/CE 显示和解析。
- **F-CAL-006 P0 内核** 年内日和星期。
- **F-CAL-007 P1 内核** ISO 周年、周数和周内日。
- **F-CAL-008 P0 内核** 日期加减整日。
- **F-CAL-009 P1 内核** 日期加减月/年及月底夹取、报错策略。
- **F-CAL-010 P0 内核** Gregorian↔Julian 同一日转换。
- **F-CAL-011 P1 内核** 地区改革日预设作为可选数据，不改变推算历法默认语义。
- **F-CAL-012 P2 适配** 其他民用/宗教历法通过独立适配器接入，不进入天文时间内核。
- **F-CAL-013 P1 内核** 分开计算跨越的历月边界数、完整历月数以及完整历月后的剩余整日，不以一个无语义的“月份差”混合三种答案。
- **F-CAL-014 P1 内核** 历法跨度携带年、月、日分量和月底调整策略；历年、历月不得隐式转换为固定秒数 `Duration`。

当前 `Date<C>` 已支持公历/儒略历验证、月长、闰年、年内日、星期、整日加法和整日差；`CalendarYears`、`CalendarMonths` 与 `CalendarSpan` 保留非固定历法分量，`InvalidDayPolicy` 显式选择拒绝或夹取月底，日期接口同时提供历月边界数、完整历月数和“完整月 + 剩余日”分解。

### 6.2 日序和历元

- **F-DATE-001 P0 内核** Julian Day Number 与日内分数。
- **F-DATE-002 P0 内核** 带时间尺度的 JD。
- **F-DATE-003 P0 内核** 带时间尺度的 MJD。
- **F-DATE-004 P1 内核** RJD、TJD 等常见偏移日数的显式类型或转换。
- **F-DATE-005 P0 内核** 两段式 JD，支持任意合法拆分。
- **F-DATE-006 P0 内核** J2000 拆分、MJD 拆分和整数日加分数拆分。
- **F-DATE-007 P0 内核** 日期时间↔两段式 JD。
- **F-DATE-008 P0 内核** 儒略世纪/千年相对参考纪元。
- **F-DATE-009 P0 内核** Julian Epoch 构造、读取和转换。
- **F-DATE-010 P0 内核** Besselian Epoch 构造、读取和转换。
- **F-DATE-011 P0 内核** J2000.0、B1950.0、J2016.0 等明确历元常量。
- **F-DATE-012 P1 内核** 星表参考历元和坐标 equinox 分开。
- **F-DATE-013 P0 内核** ISO 8601/RFC 3339 合法子集解析与格式化。
- **F-DATE-014 P0 内核** UTC 闰秒标签 `23:59:60`。
- **F-DATE-015 P1 内核** 小数秒精度、舍入和进位策略。

## 7. 时间尺度与时间数据

### 7.1 时间尺度

- **F-TIME-001 P0 内核** TAI 连续原子时间。
- **F-TIME-002 P0 数据** UTC，包括闰秒和 1972 年前漂移段策略。
- **F-TIME-003 P0 内核** TT 与 `TT = TAI + 32.184 s`。
- **F-TIME-004 P0 数据** UT1，由 EOP 的 UT1−UTC 实现。
- **F-TIME-005 P0 内核** TCG 及相对 TT 的线性漂移。
- **F-TIME-006 P0 内核** TCB 及相对 TDB 的比例关系。
- **F-TIME-007 P0 内核** TDB 低阶解析近似。
- **F-TIME-008 P1 适配** 使用历表和观测者状态的高精度 TDB−TT。
- **F-TIME-009 P1 数据** TT(BIPM) 实现标签和偏差表适配。
- **F-TIME-010 P2 内核** UT0/UT2 作为历史时间尺度，显式标记遗留用途。
- **F-TIME-011 P1 内核** TDT、ET 等历史名称只作为受控解析别名，不形成含糊新尺度。
- **F-TIME-012 P0 工作流** 任意受支持尺度之间的正确转换图。
- **F-TIME-013 P0 工作流** 转换结果保留目标时间尺度，覆盖不足返回错误。
- **F-TIME-014 P0 工作流** 缺失闰秒/EOP/历表时返回具体错误。

当前 `GeocentricTdb` 已超过 F-TIME-007 的低阶近似要求：默认 `std` 路径使用 SOFA 完整 Fairhead–Bretagnon (1990) 解析级数，返回绑定物理历元的 `TdbSolution`、双分量 `JulianDate<Tdb>` 和 `TDB−TT`。SOFA 给出的 1950–2050 年地心精度界为相对数值时间历表优于 ±3 ns；模型在区间外仍可求值但不声明该精度。F-TIME-008 的站心项及历表积分仍保持独立后续能力。

### 7.2 闰秒和偏移

- **F-LEAP-001 P0 数据** 版本化闰秒表。
- **F-LEAP-002 P0 适配** IERS Bulletin C 或标准 leap-seconds.list 解析。
- **F-LEAP-003 P0 数据** 查询 TAI−UTC。
- **F-LEAP-004 P0 内核** 正闰秒的构造、解析、排序和时长差。
- **F-LEAP-005 P1 内核** 负闰秒规则，尽管历史尚未发生。
- **F-LEAP-006 P0 数据** 闰秒表覆盖和过期检查。
- **F-LEAP-007 P0 内核** UTC 标签歧义/不存在诊断。
- **F-LEAP-008 P1 数据** 1960–1971 UTC 分段线性偏移。
- **F-LEAP-009 P0 内核** 物理时长与民用标签差分分开。

### 7.3 EOP 和 Delta T

- **F-EOP-001 P0 适配** IERS finals2000A 解析。
- **F-EOP-002 P0 适配** IERS Bulletin A 解析。
- **F-EOP-003 P0 适配** IERS Bulletin B/C04 类最终值解析。
- **F-EOP-004 P0 数据** UT1−UTC。
- **F-EOP-005 P0 数据** 极移 `xp`、`yp`。
- **F-EOP-006 P0 数据** 天极偏差 `dX`、`dY`。
- **F-EOP-007 P0 数据** LOD 和可选地球角速度修正。
- **F-EOP-008 P1 数据** 每字段标准差和相关系数。
- **F-EOP-010 P0 内核** EOP 插值，不跨闰秒错误插值 UT1−UTC。
- **F-EOP-012 P0 工作流** Delta T = TT−UT1。
- **F-EOP-013 P1 适配** 古代和未来 Delta T 经验模型。
- **F-EOP-015 P1 数据** EOP 表合并、优先级和重复日期检查。

当前 `EarthRotationTable` 和完整 `EarthOrientationTable` 都通过 `TimeContext::delta_t_at` 从同一物理历元的 `UT1−UTC`、适用的 `TAI−UTC` 与精确定义 `TT−TAI = 32.184 s` 组合 F-EOP-012，并在 UTC 闰秒处保持 `TT−UT1` 连续。未来或历史场景可改用 `DeltaTModel`：显式闭区间常量适合复现已发布预测，内置 NASA/Espenak–Meeus 2006 分段多项式覆盖 −1999—3000 年，调用者函数接缝可承载其他版本化模型；每次求值返回可选标准不确定度，不伪造模型误差。`PredictedEarthOrientation` 把该模型与具名极移/天极偏差预测或零假设组合，直接从 TT 推导 UT1，不要求也不制造未来 `UTC`、闰秒或 `UT1−UTC`。

### 7.4 GPS、Unix 与其他系统时间

- **F-SYS-001 P0 内核** GPS 时间与 TAI 固定关系。
- **F-SYS-002 P0 内核** GPS 周、周内秒和连续秒。
- **F-SYS-003 P0 内核** GPS 10 位周滚回的显式消歧。
- **F-SYS-004 P0 内核** Unix/POSIX 秒，明确不编码闰秒。
- **F-SYS-005 P1 内核** 有符号 Unix 时间和纳秒部分。
- **F-SYS-006 P1 内核** NTP 时间戳和 era 消歧。
- **F-SYS-007 P2 内核** Galileo System Time、BeiDou Time 和 QZSS 时间适配。
- **F-SYS-008 P1 适配** IANA 时区的可选民用时间适配。
- **F-SYS-009 P1 内核** 固定 UTC offset。
- **F-SYS-010 P1 工作流** 本地时间重复/跳过的策略化解析。


## 8. 岁差、章动与地球定向

### 8.1 岁差章动模型

- **F-PN-001 P0 内核** IAU 2006 岁差。
- **F-PN-002 P0 内核** IAU 2000A 完整章动。
- **F-PN-003 P0 内核** IAU 2000B 截断章动。
- **F-PN-004 P0 内核** IAU 2006/2000A 匹配修正。
- **F-PN-005 P0 内核** frame bias。
- **F-PN-006 P0 内核** precession-bias 矩阵。
- **F-PN-007 P0 内核** nutation 矩阵。
- **F-PN-008 P0 内核** precession-nutation 矩阵。
- **F-PN-009 P0 内核** Fukushima-Williams 角。
- **F-PN-010 P0 内核** 平黄赤交角和真黄赤交角。
- **F-PN-011 P0 内核** 章动经度与章动交角。
- **F-PN-012 P1 内核** IAU 1976/1980 遗留模型，限定兼容用途。
- **F-PN-013 P1 内核** 模型能力和适用年代元数据。

### 8.2 CIO/CIP/TIO

- **F-CIP-001 P0 内核** CIP 的 GCRS 坐标 `X`、`Y`。
- **F-CIP-002 P0 内核** CIO locator `s`。
- **F-CIP-003 P0 内核** TIO locator `s′`。
- **F-CIP-004 P0 数据** 应用 `dX`、`dY` 天极偏差。
- **F-CIP-005 P0 内核** `X,Y,s` 到天球中间矩阵。
- **F-CIP-006 P0 内核** CIO-based celestial-to-intermediate 变换。
- **F-CIP-007 P0 内核** equinox-based celestial-to-true 变换。
- **F-CIP-008 P0 内核** 两条链的等价校验。
- **F-CIP-009 P2 数据** 自由核章动修正适配。

### 8.3 地球旋转角与恒星时

- **F-SID-001 P0 内核** Earth Rotation Angle。
- **F-SID-002 P0 内核** IAU 2006 GMST。
- **F-SID-003 P0 内核** GAST。
- **F-SID-004 P0 内核** equation of equinoxes。
- **F-SID-005 P0 内核** equation of origins。
- **F-SID-006 P0 内核** 地方平恒星时。
- **F-SID-007 P0 内核** 地方视恒星时。
- **F-SID-008 P1 内核** ERA/恒星时角速度。
- **F-SID-009 P1 工作流** 指定恒星时反求近似 UT1，为事件求根提供初值。
- **F-SID-010 P0 内核** TT 和 UT1 参数在类型上分开。
- **F-SOLAR-TIME-001 P1 工作流** 格林尼治平太阳时由 UT1 日内分数定义，不以 UTC 近似。
- **F-SOLAR-TIME-002 P1 工作流** 真太阳时由日期真赤道与真分点轴上的太阳视赤经和 GAST 得到，定义为真太阳当地时角加 12 小时。
- **F-SOLAR-TIME-003 P1 内核** 时差固定定义为“真太阳时减平太阳时”，返回 `(-12h, 12h]` 内的强类型有符号时长。
- **F-SOLAR-TIME-004 P1 内核** 以东经为正把格林尼治平/真太阳时转换为地方平/真太阳时；经度不改变时差。
- **F-SOLAR-TIME-005 P1 工作流** 太阳时结果保留同一次地心太阳视位置、UT1、GAST、光行时次数和残差，不混入时区或民用时间。

当前实现由 `EarthRotationSample` / `EarthRotationTable` 提供只含 `UT1−UTC` 的数据能力，`Frames::sidereal_time_at` 返回不可变 `SiderealTimeSolution`。ERA、GMST、GAST、地方平恒星时和地方视恒星时不依赖完整 EOP；`Astrometry::solar_time` 组合 `SolarApparentPlace` 与该恒星时结果，返回强类型 `SolarTimeSolution`、地方平/真太阳时和时差。`EarthOrientationTable` 仍用于需要 LOD、极移和天极偏差的完整地球姿态与状态变换。

### 8.4 极移与完整地球链

- **F-PM-001 P0 内核** `xp`、`yp` 极移矩阵。
- **F-PM-002 P0 内核** 应用 TIO locator `s′`。
- **F-PM-003 P0 工作流** GCRS→CIRS。
- **F-PM-004 P0 工作流** CIRS→TIRS，应用 ERA。
- **F-PM-005 P0 工作流** TIRS→ITRS，应用极移。
- **F-PM-006 P0 工作流** GCRS→ITRS 合成旋转。
- **F-PM-007 P0 工作流** 所有逆向变换。
- **F-PM-008 P0 工作流** 位置与方向变换。
- **F-PM-009 P0 工作流** 带角速度的状态和速度变换。
- **F-PM-010 P1 数据** EOP 不确定度传播。
- **F-PM-011 P2 数据** 亚日潮汐对 UT1、LOD、极移的修正。
- **F-PM-012 P2 数据** 地球潮汐导致的站点位移。

## 9. 参考系统、参考架与坐标系统

### 9.1 天球与相对论参考系统

- **F-FRM-001 P0 内核** ICRS 方向和状态语义。
- **F-FRM-002 P0 内核** BCRS 原点、时间和状态语义。
- **F-FRM-003 P0 内核** GCRS 原点、时间和状态语义。
- **F-FRM-004 P0 内核** CIRS 中间赤经赤纬。
- **F-FRM-005 P0 内核** TIRS。
- **F-FRM-006 P0 内核** ITRS。
- **F-FRM-007 P1 数据** ICRF1/2/3 实现标签和源元数据。
- **F-FRM-008 P1 数据** ITRF 具体版本标签、参考历元和速度。
- **F-FRM-009 P1 数据** ITRF 版本间 7/14 参数 Helmert 变换。

### 9.2 历史和天文坐标系

- **F-FRM-010 P0 内核** FK5/J2000。
- **F-FRM-011 P0 内核** FK4/B1950，包括 E-terms。
- **F-FRM-012 P1 内核** FK4 无 E-terms 表示。
- **F-FRM-013 P0 工作流** FK4↔FK5，包含位置、自行、历元和径向运动。
- **F-FRM-014 P0 内核** IAU Galactic 坐标。
- **F-FRM-015 P1 内核** Supergalactic 坐标。
- **F-FRM-016 P0 内核** J2000 平黄道坐标。
- **F-FRM-017 P0 内核** 日期平黄道坐标。
- **F-FRM-018 P1 内核** 日期真黄道采用命名的 `TrueEclipticEquinoxOfDate`：以 IAU 2006 frame bias/岁差、IAU 2000A 章动和真黄赤交角 $\epsilon_A+\Delta\epsilon$ 固定黄道面与真分点；类型只表示轴和历元，不暗示空间原点或视位置修正。
- **F-FRM-019 P0 内核** 黄道模型和历元成为类型/元数据。
- **F-FRM-020 P1 内核** MOD、TOD、PEF 遗留地球卫星参考系。
- **F-FRM-021 P1 内核** TEME 及其明确转换约定。

### 9.3 局部和轨道参考系

- **F-FRM-030 P0 内核** `TopocentricFrame<S>` 把固定站点的 GCRS 状态、ENU 基和物理历元冻结为一个运行时站心参考架。
- **F-FRM-031 P0 内核** `EastNorthUp<F>` 及站心方向往返。
- **F-FRM-032 P1 内核** `NorthEastDown<F>` 及与 ENU 一致的历元变换。
- **F-FRM-033 P0 内核** `HorizontalDirection` 使用北起东增方位/高度语义；天顶和天底的方位明确为 `None`。
- **F-FRM-034 P0 内核** Hour-angle/declination。
- **F-FRM-035 P1 内核** parallactic frame 和视场旋转。
- **F-FRM-036 P1 内核** RTN/RSW 轨道局部架。
- **F-FRM-037 P1 内核** TNW/VNB/LVLH 轨道架并明确各自轴定义。
- **F-FRM-038 P1 内核** 动态参考系注册表和路径搜索。
- **F-FRM-039 P1 内核** 路径组合、循环检测和能力查询。

### 9.4 天体固连参考系

- **F-BFRM-001 P2 数据** IAU/WGCCRE 天体北极和首子午线模型。
- **F-BFRM-002 P2 内核** 惯性↔天体固连姿态。
- **F-BFRM-003 P2 适配** SPICE PCK 文本/二进制姿态。
- **F-BFRM-004 P2 适配** SPICE FK 自定义参考系。
- **F-BFRM-005 P2 内核** 天体表面经纬度与三轴椭球。
- **F-BFRM-006 P2 数据** 姿态覆盖、模型版本和角速度。

## 10. 星表数据与空间运动

### 10.1 通用星表模型

- **F-CAT-002 P0 内核** 参考历元和参考系。
- **F-CAT-003 P0 内核** 赤经、赤纬。
- **F-CAT-004 P0 内核** 年周视差及误差。
- **F-CAT-005 P0 内核** `μα*` 与 `μδ` 自行。
- **F-CAT-006 P0 内核** 未乘 `cos δ` 的赤经自行显式转换。
- **F-CAT-007 P0 内核** 径向速度、符号和定义。
- **F-CAT-008 P1 内核** 五参数/六参数协方差。
- **F-CAT-009 P1 内核** 相关系数↔协方差。
- **F-CAT-010 P1 内核** 光度、颜色和星等的扩展记录。
- **F-CAT-011 P1 内核** 缺失字段、上限和无效值分开表示。
- **F-CAT-012 P1 内核** 二元/多星和非单星解类型。

### 10.2 Gaia DR3

- **F-GAIA-001 P1 适配** `gaia_source` CSV/ECSV 流式解析。
- **F-GAIA-002 P1 适配** VOTable 解析。
- **F-GAIA-003 P2 适配** FITS 表解析。
- **F-GAIA-004 P2 适配** Parquet/Arrow 解析。
- **F-GAIA-006 P1 适配** 2 参数、5 参数和 6 参数解识别。
- **F-GAIA-007 P1 适配** `ref_epoch` 按 TCB Julian year 解释。
- **F-GAIA-008 P1 适配** RA、Dec、parallax、pmra、pmdec 和 radial_velocity。
- **F-GAIA-009 P1 适配** 全部天体测量误差和相关系数。
- **F-GAIA-011 P2 适配** 光度、变星、非单星和太阳系对象关联表。
- **F-GAIA-013 P1 适配** 列选择和未知列向前兼容。
- **F-GAIA-014 P1 适配** 行号、列名和原始值解析错误。

### 10.3 其他星表和格式

- **F-CATFMT-001 P2 适配** Hipparcos 新/旧减表。
- **F-CATFMT-002 P2 适配** Tycho-2。
- **F-CATFMT-003 P2 适配** FK4/FK5 基本星表。
- **F-CATFMT-004 P2 适配** MPC 小行星/彗星目录。
- **F-CATFMT-005 P1 适配** 用户自定义 CSV 列映射和单位声明。
- **F-CATFMT-006 P1 适配** 流式读取和有界内存。
- **F-CATFMT-007 P1 数据** 原始目录版本、校验和和查询条件。

### 10.4 空间运动和历元传播

- **F-MOTION-001 P0 内核** 星表参数→BCRS/ICRS 三维位置速度。
- **F-MOTION-002 P0 内核** 三维状态→星表参数。
- **F-MOTION-003 P0 内核** 直线空间运动传播。
- **F-MOTION-004 P0 内核** 透视加速度。
- **F-MOTION-005 P0 内核** 位置、视差、自行和径向速度联合传播。
- **F-MOTION-006 P0 工作流** 任意目标历元传播和反向传播。
- **F-MOTION-007 P1 内核** 缺失径向速度的假设策略。
- **F-MOTION-008 P1 内核** 缺失或非正视差的角位置传播策略。
- **F-MOTION-009 P1 内核** 超光速/不一致输入诊断。
- **F-MOTION-010 P1 内核** 参数 Jacobian 和协方差传播。
- **F-MOTION-011 P1 内核** 传播后相关系数恢复。
- **F-MOTION-012 P1 工作流** 大批量星表同历元传播。

当前实现覆盖 F-MOTION-001–006、010–011 的单源核心路径：`SpatialCatalogPlace` 用 TCB 参考历元、ICRS 方向、$\mu_{\alpha *}$ / $\mu_\delta$、严格正周年视差和质心天体测量径向速度表达六参数星表位置；`BarycentricCatalogState` 表达同一物理量的 SSB/ICRS 三维位置速度。两者用 SOFA `starpv` / `pvstar` 双向转换，`propagate_to` 用 SOFA `starpm` 联合传播位置、视差、自行及径向速度，并支持反向历元传播。SOFA 为缺失/极小视差、超速或不收敛输入启用的替代结果不会静默进入强类型值，而是返回带状态位的错误。带不确定度且可为零或负的 `ParallaxMeasurement` 与严格正 `Parallax` 分离。`SpatialCatalogPlaceWithCovariance` 在固定的 $\alpha*$、$\delta$、$\varpi$、$\mu_{\alpha *}$、$\mu_\delta$、$v_r$ 顺序和规范单位下，以五点数值 Jacobian 执行 $J C J^\mathsf{T}$，并恢复传播后的标准不确定度及相关系数。F-MOTION-007–009 和 F-MOTION-012 尚未落地。

可运行星表示例：

- `spatial_catalog_motion`：对 SOFA 六参数参考星执行星表参数→质心状态→星表参数往返，以及目标历元正向/反向传播。
- `spatial_catalog_observation`：读取调用者提供的 DE BSP 与 IERS C04，把有限距离六参数源传播到接收历元，并通过固定站点真空观测链输出站心视差和地平方向。

## 11. 天体测量和观测位置

### 11.1 位置层级

- **F-PLACE-001 P0 内核** Catalog place。
- **F-PLACE-002 P0 内核** Geometric state/place。
- **F-PLACE-003 P0 内核** Astrometric place。
- **F-PLACE-004 P0 内核** Apparent place。
- **F-PLACE-005 P0 内核** Observed place。
- **F-PLACE-006 P0 内核** 结果携带已应用修正清单。
- **F-PLACE-007 P0 内核** 防止同一修正重复应用。
- **F-PLACE-008 P1 工作流** 每一步中间结果和诊断分解。

### 11.2 光传播和相对论效应

- **F-LIGHT-001 P0 工作流** BCRS 单程接收光行时迭代：固定接收时刻观测者，迭代目标发射时刻，并返回双历元、距离、方向、次数和时间残差。
- **F-LIGHT-002 P1 工作流** 单程发射光行时。
- **F-LIGHT-003 P1 工作流** 双程 uplink/downlink 光行时。
- **F-LIGHT-004 P0 内核** Roemer 几何延迟。
- **F-LIGHT-005 P0 内核** 太阳有限距离单极引力偏折；必须区分目标即太阳、太阳盘后被遮挡的目标中心和太阳盘前的有限距离目标。
- **F-LIGHT-006 P1 内核** 木星、土星、地球和多体引力偏折。
- **F-LIGHT-007 P1 内核** Shapiro 延迟。
- **F-LIGHT-008 P0 内核** 使用观测者相对 SSB 速度和日心距离的 SOFA 相对论光行差。
- **F-LIGHT-009 P0 内核** 固定站点路径以地球质心速度与 EOP 驱动的站点 GCRS 速度组合周日和周年光行差。
- **F-LIGHT-010 P0 内核** 相对论速度变换。
- **F-LIGHT-011 P1 内核** 掩蔽引力体筛选和近边缘稳定性；太阳单体路径必须按版本化太阳半径判断不透明盘面，并在盘面外调用稳定的 SOFA 有限源公式。
- **F-LIGHT-012 P0 工作流** 有限距离太阳系目标与无限远恒星采用不同路径。

有限太阳系目标的当前落地范围：`FixedObserverAt<S>` 固定站点接收状态并复用 SOFA 星无关参数；`vacuum_observed_place` 迭代目标发射时刻，按光线最接近太阳的历元应用 SOFA `ld` 有限源太阳单极偏折，再应用相对论光行差，并返回双历元、距离、CIRS 方向、真空地平方向、偏折分类与收敛诊断。太阳目标明确标记为“目标即偏折体”而不自偏折；太阳盘后的目标中心明确标记为被遮挡而不把受限公式伪装成观测结果。`VacuumObservedPlace::apply_refraction` 消耗真空阶段并返回不可再次折射的 `ObservedPlace<S>`。星表源采用独立的参数化链：无限远源使用 `InfiniteCatalogPlace → AstrometricCatalogPlace<S> → VacuumObservedCatalogPlace<S> → ObservedCatalogPlace<S>`，六参数有限距离源使用对应的 `SpatialCatalogPlace` / `AstrometricSpatialCatalogPlace<S>` / `VacuumObservedSpatialCatalogPlace<S>` / `ObservedSpatialCatalogPlace<S>` 类型。两者都应用观测者相关 Roemer 项、`ldsun` 远源太阳偏折、组合站点速度光行差、IAU 2006/2000A、地球定向、真空地平投影和可选折射；空间源另按完整 SSB 到站点基线应用周年及周日视差。星表结果不伪造太阳系目标式发射历元或迭代光行时。多体偏折与 Shapiro 延迟仍未落地。

地心有限目标路径由 `Astrometry::geocentric_apparent_place` 提供统一入口：地球固定在接收历元、目标迭代到发射历元，随后按光线近太阳历元应用同一有限源太阳单极偏折，再应用地球质心周年光行差，并返回保留目标身份、双历元、距离、GCRS、日期真赤道、日期真黄道、偏折处置和收敛证据的 `GeocentricApparentPlace<S>`。`solar_apparent_place` 复用该路径并以 `SolarApparentPlace<S>` 保留太阳专用语义；太阳自偏折明确记为 `NotAppliedToSun`。

### 11.3 地球和站心修正

- **F-OBSPLACE-001 P0 工作流** BCRS 自然视线→GCRS proper direction。
- **F-OBSPLACE-002 P0 工作流** GCRS→同一接收历元的 CIRS 中间赤道方向。
- **F-OBSPLACE-003 P0 工作流** CIRS→TIRS 地球自转方向链。
- **F-OBSPLACE-004 P0 工作流** TIRS→ITRS 极移方向链。
- **F-OBSPLACE-005 P0 工作流** 地心视差。
- **F-OBSPLACE-006 P0 工作流** 有限目标使用“目标发射位置−站点接收质心位置”的站心视差。
- **F-OBSPLACE-007 P0 工作流** 同一 `TopocentricFrame<S>` ENU 基上的几何方位高度。
- **F-OBSPLACE-008 P1 工作流** 大气折射后观测方位高度。
- **F-OBSPLACE-009 P1 工作流** 观测位置反算 CIRS/ICRS。
- **F-OBSPLACE-010 P1 内核** parallactic angle。
- **F-OBSPLACE-011 P1 内核** field rotation 和变化率。
- **F-OBSPLACE-012 P1 内核** 大气质量。

当前 `ObservedPlace<S>` 路径实现 F-OBSPLACE-008：调用者必须显式提供 `AtmosphericConditions`，生产计算使用 `sofars 0.6.1` 的 SOFA `refco` 系数与 `atioq` CIRS→观测坐标链，并返回模型适用范围分类；结果保留来源真空阶段、双历元、距离和光行时诊断。`VacuumObservedPlace`、`ObservedPlace` 及星表对应阶段现可返回历元绑定的 `ParallacticAngleAt<S>`，采用 SOFA `iauHd2pa` 的有符号约定。`Astrometry::field_rotation_at` 在请求历元前后各完成一次独立的站点/EOP/目标光行时链，用对称差分返回 `FieldRotation<S>` 的中心视差角、有符号变化量、角速度、方向和采样证据；完整 `EarthOrientationTable` 路径使用观测 LOD，任意 `EarthAttitudeModel` 的降级接口明确命名为名义地球自转率。当前覆盖 F-OBSPLACE-010/011；反算和气团质量 F-OBSPLACE-009/012 仍未落地。

### 11.4 Barycentric 校正

- **F-BARY-001 P1 工作流** HJD 及明确时间尺度。
- **F-BARY-002 P1 工作流** BJD_TDB。
- **F-BARY-003 P1 工作流** BJD 的 Roemer、Einstein、Shapiro 分解。
- **F-BARY-004 P1 工作流** barycentric radial-velocity correction。
- **F-BARY-005 P1 内核** 光学径向速度定义。
- **F-BARY-006 P1 内核** 射电径向速度定义。
- **F-BARY-007 P1 内核** 相对论多普勒定义。
- **F-BARY-008 P2 工作流** 曝光时间权重、曝光中点和长曝光积分。

## 12. 观测者与大地测量

### 12.1 地球椭球和地理坐标

- **F-GEO-001 P0 内核** 可配置旋转椭球：长半轴和扁率。
- **F-GEO-002 P0 数据** WGS84。
- **F-GEO-003 P1 数据** GRS80。
- **F-GEO-004 P1 数据** 常用历史椭球。
- **F-GEO-005 P0 内核** 测地经度、测地纬度、椭球高。
- **F-GEO-006 P0 内核** 地心纬度。
- **F-GEO-007 P1 内核** 正高和大地水准面高分开。
- **F-GEO-008 P0 内核** 测地坐标→ITRS 地心直角坐标。
- **F-GEO-009 P0 内核** ITRS 地心直角坐标→测地坐标。
- **F-GEO-010 P0 内核** 极点、地心、地表内和高空稳定性。
- **F-GEO-011 P1 适配** 大地线正解。
- **F-GEO-012 P1 适配** 大地线反解。
- **F-GEO-013 P2 数据** 大地水准面/重力场适配。

### 12.2 站点和移动观测者

- **F-SITE-001 P0 内核** `FixedSite` 保存标识、参考椭球、测地位置、ITRS 位置和局部基。
- **F-SITE-002 P1 内核** 站点参考历元和速度。
- **F-SITE-003 P1 数据** 架站坐标不连续和地震跳变。
- **F-SITE-004 P0 工作流** 完整 EOP 状态变换把固定 ITRS 站点转换为含地球自转速度的 GCRS 状态。
- **F-SITE-005 P0 工作流** `FixedObserverAt<S>` 将站点 GCRS 状态与地球历表状态组合为 BCRS 接收位置/速度。
- **F-SITE-006 P0 内核** 类型化 ENU/NED 基及 GCRS 历元快照。
- **F-SITE-007 P0 内核** `TopocentricFrame<S>` 的极移、地球自转和局部子午线均来自同一次完整 EOP 求值。
- **F-SITE-008 P1 内核** 移动观测者轨迹接口。
- **F-SITE-009 P1 适配** 航天器历表作为观测者。
- **F-SITE-010 P1 数据** 温度、气压、湿度和波长配置。
- **F-SITE-011 P1 数据** 方位相关地平线剖面。
- **F-SITE-012 P2 数据** 数字高程地形地平线。
- **F-SITE-013 P2 数据** 板块运动。
- **F-SITE-014 P2 数据** 固体地球潮、海潮负荷、极潮、大气负荷。

当前实现由 `ReferenceEllipsoid`、`Earth`、`GeodeticPosition` 和 `FixedSite` 提供 F-GEO-001/002/003/005/006/008/009/010 与 F-SITE-001/004/006/007。WGS 84、GRS 80 和自定义椭球均显式绑定；SOFA Fukushima (2006) 路径完成测地坐标与 `Point3<Itrs>` 双向转换，地心原点明确返回未定义错误。固定站点保存零 ITRS 速度及 ENU/NED 基。完整 EOP 路径使用 LOD 和可用帧率求 GCRS 状态；不含 LOD 的姿态表路径保留 UT1、极移和天极偏差，但只经显式入口使用 IERS 名义自转率。两种 `TopocentricFrame` 均携带可查询的 `SiteVelocityModel`，不会把缺失 LOD 伪造成零或观测量。`AtmosphericConditions` 提供独立的逐次观测环境输入，但不把会随时间变化的气象值固化进 `FixedSite`；站点速度/不连续事件和移动观测者仍由后续条目负责。

## 13. 大气折射与传播介质

### 13.1 光学折射

- **F-REF-001 P1 内核** 温度、气压、湿度和波长输入。
- **F-REF-002 P1 内核** 真高度→视高度。
- **F-REF-003 P1 内核** 视高度→真高度迭代逆解。
- **F-REF-004 P1 内核** 快速双系数/经验近似。
- **F-REF-005 P2 内核** 分层大气数值积分。
- **F-REF-006 P1 内核** 光学和近红外折射率。
- **F-REF-007 P1 内核** 色散和差分折射。
- **F-REF-008 P1 内核** 地平线附近和模型适用范围错误。
- **F-REF-009 P1 内核** 折射梯度和高度变化率。
- **F-REF-010 P1 内核** 气团质量和光程因子。

当前已实现 F-REF-001/002/004 的前向观测路径：`AtmosphericPressure`、`AirTemperature`、`RelativeHumidity` 和 `ObservingWavelength` 在进入 SOFA 前拒绝非有限值及模型区间外值；压力为零显式表示真空，不存在默认大气。`RefractionCorrection` 返回真空减观测天顶距的有符号角量和 `Nominal`、`HighZenithDistance`、`NearHorizon`、`BelowHorizon` 适用范围分类。光学/红外与射电分支由波长按 SOFA 的 100 μm 边界选择；逆解、分层积分、独立差分折射与气团质量仍未实现。

### 13.2 对流层

- **F-TROP-001 P2 内核** 静力天顶延迟。
- **F-TROP-002 P2 内核** 湿天顶延迟。
- **F-TROP-003 P2 内核** Saastamoinen 类模型。
- **F-TROP-004 P2 数据** GPT/GPT2 类全球气象模型。
- **F-TROP-005 P2 数据** VMF 类映射函数。
- **F-TROP-006 P2 内核** 高度角映射、南北/东西梯度。
- **F-TROP-007 P2 工作流** 单程、双程测距和 VLBI/GNSS 延迟。
- **F-TROP-008 P2 数据** 气象输入和覆盖检查。

### 13.3 电离层

- **F-ION-001 P2 内核** 一阶 TEC/频率群延迟。
- **F-ION-002 P2 内核** 相位提前。
- **F-ION-003 P2 内核** 垂直 TEC→斜向 TEC 映射。
- **F-ION-004 P2 内核** 双频无电离层组合。
- **F-ION-005 P2 适配** IONEX 解析和时空插值。
- **F-ION-006 P2 内核** 二阶地磁场相关项。
- **F-ION-007 P2 数据** TEC 覆盖和太阳活动输入。

## 14. 历表统一接口

### 14.1 查询模型

- **F-EPH-001 P0 内核** 目标、中心、历元和参考系组成查询。
- **F-EPH-002 P0 内核** 位置查询。
- **F-EPH-003 P0 内核** 位置速度状态查询。
- **F-EPH-004 P0 内核** 目标和中心使用 hyastro 天体类型。
- **F-EPH-005 P0 内核** 太阳系质心、太阳、行星质心、行星、月球和冥王星。
- **F-EPH-006 P1 内核** 小行星、彗星、卫星和用户目标。
- **F-EPH-007 P0 数据** 覆盖区间查询。
- **F-EPH-008 P0 数据** 目标/中心/参考系能力枚举。
- **F-EPH-010 P0 工作流** 多后端组合和显式优先级。
- **F-EPH-011 P0 工作流** 中心链解析和循环检测。
- **F-EPH-012 P0 工作流** 覆盖外、目标缺失、参考系缺失和后端错误分开。
- **F-EPH-013 P1 内核** 单时刻无分配查询。
- **F-EPH-014 P1 内核** 批量时刻和批量目标查询。
- **F-EPH-015 P1 内核** 调用者拥有的确定性缓存。

当前 `EphemerisProvider` 已把单后端接缝收敛为三项能力：`state(EphemerisQuery<Bcrs, S>)`、连续 `coverage` 和可保留的 `EphemerisProvenance`。`Astrometry`、`FixedObserverAt` 与 `Events` 通过提供者类型参数静态分派，不再依赖 ANISE 具体类型；外部测试后端可只使用 hyastro 的查询、状态、覆盖和错误类型实现同一接缝。当前实现覆盖 F-EPH-001 至 F-EPH-004、F-EPH-007 和 F-EPH-012 的单后端路径；F-EPH-008 的统一能力枚举、F-EPH-010 的多后端组合/显式优先级，以及批量和缓存仍未实现。后端缺失或覆盖外不会触发隐式回退。

### 14.2 DAF/SPK/BSP

- **F-ANISE-001 P0 适配** 采用 ANISE 0.10 系列实现 hyastro 历表和动态参考系接缝。
- **F-ANISE-002 P0 适配** 使用 `default-features = false`，默认关闭 `metaload`、`embed_ephem` 和 `analysis`。
- **F-ANISE-003 P0 适配** hyastro 的时间、参考系、原点、位置和状态类型与 ANISE `Almanac`/`Frame`/`Orbit` 双向受检转换。
- **F-ANISE-004 P0 适配** SPK 类型 1、2、3、9、13 的查询、覆盖和段优先级。
- **F-ANISE-005 P2 适配** SPK 类型 8、12 在获得充分公开内核验证前不对外开放。
- **F-ANISE-006 P0 适配** BPC 姿态与文本 PCK/FK/TPC 转换后的 PCA/EPA/LKA 数据加载。
- **F-ANISE-007 P0 适配** 调用者显式提供本地内核及冻结加载顺序；不要求 SHA-256。hyastro 核心禁止 ANISE 自动下载及 “latest” 数据。
- **F-ANISE-008 P0 适配** CK、SCLK、DSK、IK、EK 和不支持的 SPK 类型返回结构化能力错误。
- **F-SPK-001 P0 适配** DAF 文件记录、字节序和文件标识。
- **F-SPK-002 P0 适配** 摘要记录和名称记录。
- **F-SPK-003 P0 适配** 注释区读取。
- **F-SPK-004 P0 适配** 段目标、中心、参考系、类型和覆盖。
- **F-SPK-005 P0 适配** 段逆序优先级和多文件优先级。
- **F-SPK-006 P0 适配** SPK type 2 Chebyshev position。
- **F-SPK-007 P0 适配** SPK type 3 Chebyshev position/velocity。
- **F-SPK-008 P1 适配** SPK type 1 modified difference arrays。
- **F-SPK-009 P1 适配** SPK type 5 discrete states/two-body。
- **F-SPK-010 P2 适配** SPK type 8/9 Lagrange。
- **F-SPK-011 P2 适配** SPK type 10 TLE/SGP4 数据。
- **F-SPK-012 P2 适配** SPK type 12/13 Hermite。
- **F-SPK-013 P2 适配** SPK type 14 unequal Chebyshev。
- **F-SPK-014 P2 适配** SPK type 15 precessing conic。
- **F-SPK-015 P2 适配** SPK type 17 equinoctial elements。
- **F-SPK-016 P2 适配** SPK type 18/19 ESOC/DDID。
- **F-SPK-017 P2 适配** SPK type 20 velocity Chebyshev。
- **F-SPK-018 P2 适配** SPK type 21 extended MDA。
- **F-SPK-019 P0 适配** DE440/DE441 常用目标链。
- **F-SPK-020 P0 适配** 覆盖端点和记录边界语义。
- **F-SPK-021 P1 适配** 内存映射读取。
- **F-SPK-022 P1 适配** 普通 `Read + Seek` 数据源。
- **F-SPK-023 P0 适配** checked offset、长度和资源上限。
- **F-SPK-024 P0 适配** 与 CSPICE/JPL 参考结果差分。

### 14.3 解析历表

- **F-VSOP-001 P1 适配** VSOP87 原始版本。
- **F-VSOP-002 P1 适配** VSOP87A/B/C/D/E 坐标形式区分。
- **F-VSOP-003 P1 适配** Full/截断系数精度级别。
- **F-VSOP-004 P1 工作流** VSOP 日心/质心结果适配统一接口。
- **F-MOON-001 P2 适配** ELP/MPP02 类高精度月球理论。
- **F-MOON-002 P1 适配** 明确误差范围的快速月球模型。
- **F-ANA-001 P1 适配** 明确误差范围的快速太阳/行星模型。
- **F-ANA-002 P1 数据** 每个解析模型的版本、坐标、时标和有效期。

当前默认 `std` 后端 `SofaAnalyticEphemeris` 将 SOFA `epv00`（SSB/太阳/地球）、`moon98`（地心月球）和 `plan94`（水星、金星、地月、火星、木星、土星、天王星、海王星系统质心）接入同一 `EphemerisProvider`，不分配且不读取文件。日地查询采用 1900–2100 覆盖，月球采用 1950–2100 覆盖；仅由太阳和 PLAN94 系统组成的查询采用 1000–3000 覆盖，与地球、月球或 SSB 组合时取所需模型覆盖的交集。PLAN94 结果只映射到 `*Barycenter`/`EarthMoonBarycenter` 身份，不把系统质心伪装成带物理表面的行星本体。

`Plan94Accuracy` 公开 SOFA 对每个系统发布的误差合同：1800–2100 对 DE200/DE406 的最大黄经、黄纬和半径差，以及 1960–2025 对 DE200 的位置/速度 RMS。使用 DE440s 在 1900、2000、2024、2100 四个代表历元进行的日心系统质心差分中，各系统采样最大位置差依次为：水星 350 km、金星 988 km、地月质心 867 km、火星 9,125 km、木星 116,868 km、土星 289,098 km、天王星 614,028 km、海王星 196,833 km；均小于由 SOFA 已发布角度与半径分量合成的同历元位置界。该实现落地 F-MOON-002、F-ANA-001 和 F-ANA-002；VSOP87、高精度 ELP/MPP02、冥王星解析状态以及行星本体/卫星质心拆分仍未实现。

### 14.4 SPICE 生态扩展

- **F-SPICE-001 P2 适配** LSK 闰秒内核。
- **F-SPICE-002 P2 适配** PCK 行星常数和姿态。
- **F-SPICE-003 P2 适配** FK 参考系。
- **F-SPICE-004 P2 适配** CK 姿态。
- **F-SPICE-005 P2 适配** SCLK 航天器时钟。
- **F-SPICE-006 P2 适配** 内核池隔离、加载顺序和卸载。
- **F-SPICE-007 P2 适配** CSPICE 全局状态的线程安全封装。

## 15. 轨道与小天体

- **F-ORB-001 P1 内核** 笛卡尔状态↔经典轨道根数。
- **F-ORB-002 P1 内核** 圆轨道、赤道轨道和抛物线奇异性诊断。
- **F-ORB-003 P2 内核** equinoctial/modified equinoctial 根数。
- **F-ORB-004 P1 内核** 二体 Kepler 方程的椭圆、双曲和抛物解。
- **F-ORB-005 P1 内核** 通用变量传播。
- **F-ORB-006 P1 内核** 近日点、远日点、节点、轨道周期派生量。
- **F-ORB-007 P2 适配** MPCORB 和彗星根数解析。
- **F-ORB-008 P2 适配** H/G、H/G1/G2 小天体星等模型。
- **F-ORB-009 P2 适配** TLE/OMM 解析。
- **F-ORB-010 P2 适配** SGP4 传播和 TEME 语义。
- **F-ORB-011 P2 适配** 独立动力学接缝接入数值传播、力模型和状态转移矩阵；当前生产依赖不引入 Nyx。
- **F-ORB-012 P2 工作流** 轨道协方差传播和接近事件。
- **F-ORB-013 P2 数据** 小天体非引力参数和解版本。

## 16. 太阳、月球、行星物理量

- **F-PHYS-001 P1 工作流** 日心、地心、站心距离和距离变化率。
- **F-PHYS-002 P1 工作流** 太阳距和观测者距。
- **F-PHYS-003 P1 内核** 角直径和视半径。
- **F-PHYS-004 P1 内核** 相位角。
- **F-PHYS-005 P1 内核** 照亮比例。
- **F-PHYS-006 P1 内核** 亮边位置角。
- **F-PHYS-007 P1 适配** 行星经验视星等模型。
- **F-PHYS-008 P1 工作流** 太阳/月球盘面中心和边缘位置。

当前球形视盘路径使用 `SphericalBodyFigure` 绑定天体、正半径和模型版本；内置 IAU 2015 名义太阳半径与 IAU WGCCRE 2015 月球参考球。`VacuumObservedPlace::apparent_disk` 用收敛的发射目标—接收站点距离计算精确 `asin(R/Δ)` 视半径，返回保留真空中心和形状模型的 `VacuumApparentDisk`。两个同站点、同接收历元视盘可查询中心角距、带符号边缘间隙及分离/相切/部分重叠/包含分类。椭球和三轴椭球轮廓、重叠面积与接触时刻仍未落地。
- **F-PHYS-009 P1 工作流** 月球光学天平动。
- **F-PHYS-010 P2 工作流** 月球物理天平动。
- **F-PHYS-011 P1 工作流** 月面中心经纬度、位置角和轴位置角。
- **F-PHYS-012 P2 工作流** 行星中央经度、极轴位置角和盘面倾角。
- **F-PHYS-013 P2 工作流** 土星环倾角和环面位置角。
- **F-PHYS-014 P2 工作流** 行星卫星相对位置及互掩互食几何。
- **F-PHYS-015 P1 工作流** 月球经验视星等模型及适用范围诊断。

当前 `LunarRotationModel::Iau2009Wgccre` 逐项实现 NAIF `pck00011.tpc` 收录的 IAU WGCCRE 2009 月球极轴、本初子午线及 13 项周期项，并以 TDB 为独立变量。`Astrometry::lunar_disk_orientation_at` 复用三段光行时一致的 `LunarIllumination<S>`：平均旋转要素给出光学天平动，含周期项要素给出总天平动，两者的最短有符号差给出物理天平动；结果同时返回地心月面中心东经/纬度、日期真赤道北起向东量的月轴位置角及亮边位置角。模型未伪造有效区间，`applicability()` 明确提示亚角分需求应改用任务级月球姿态内核。上述路径覆盖 F-PHYS-006、F-PHYS-009—011；高精度姿态内核适配和站心周日天平动仍未实现。

当前 `HorizonsCompatibleLunarV` 消耗现有 `LunarIllumination<S>` 的三条收敛光行时和物理相位角，计算地心、无大气、积分月面 Johnson V/Vega 星等。`GeocentricLunarVMagnitude<S>` 保留原始照明几何、距离项、相位项、模型标识及 `LunarVApplicability`；相位角小于 $7^\circ$ 的已知偏暗区和月面与地影相交均不会静默标成正常结果。行星经验模型、大气消光和月食亮度衰减仍未实现，F-PHYS-007 保持待实现。

## 17. 通用事件引擎

- **F-EVT-001 P0 内核** 闭时间区间搜索。
- **F-EVT-002 P0 内核** 标量零点事件。
- **F-EVT-003 P0 内核** 上穿、下穿和无方向穿越。
- **F-EVT-004 P0 内核** 局部极小和极大。
- **F-EVT-005 P1 内核** 区间全局最小/最大候选验证。
- **F-EVT-006 P1 内核** 布尔状态进入/离开窗口。
- **F-EVT-007 P1 内核** 多接触阶段事件。
- **F-EVT-008 P0 内核** 自适应扫描与括根。
- **F-EVT-009 P0 内核** Brent 精化。
- **F-EVT-010 P0 内核** 周期角连续化。
- **F-EVT-011 P1 内核** 切触根识别。
- **F-EVT-012 P0 内核** 端点事件。
- **F-EVT-013 P0 内核** 时间与事件身份联合去重。
- **F-EVT-014 P0 内核** 最大求值次数和取消。
- **F-EVT-015 P0 内核** 残差、时间误差和迭代次数。
- **F-EVT-016 P1 内核** 确定性并行批量目标搜索。
- **F-EVT-017 P0 工作流** 整段历表/EOP/大气覆盖预检查。

当前事件数值内核包含两条共享接缝：`RootOptions::brent` 在符号括区间内精化根；内部 `BracketedExtremumSearch` 在三点括区间内用有界 Brent 搜索极小/极大值。`AngularEventSearchOptions` 与 `ExtremumSearchOptions` 分别控制扫描步长、时间/判据容差、精化迭代、历表求值上限和接收光行时。太阳节气、月相、行星配置、驻留、距离/角距/坐标极值均保留端点语义、近时刻去重及 `EventEvidence` 或 `ExtremumEvidence`。任意调用者谓词、切触根、取消和整段数据覆盖预检查仍未升格为公开通用接缝。

## 18. 地平事件和观测窗口

### 18.1 升中落

- **F-RST-001 P1 工作流** 几何中心升起。
- **F-RST-002 P1 工作流** 几何中心落下。
- **F-RST-003 P1 工作流** 上中天。
- **F-RST-004 P1 工作流** 下中天。
- **F-RST-005 P1 工作流** 任意高度上穿和下穿。
- **F-RST-006 P1 工作流** 太阳上边缘和标准折射阈值。
- **F-RST-007 P1 工作流** 月球动态视半径、视差和折射。
- **F-RST-008 P1 工作流** 恒星/行星自定义盘面边缘。
- **F-RST-009 P1 工作流** 方位相关地平线剖面。
- **F-RST-010 P1 工作流** 极昼、极夜、拱极、不升和不落分类。
- **F-RST-011 P1 工作流** 一日多次穿越和不规则地平线。
- **F-RST-012 P1 工作流** 中天时高度、方位和时角。

当前实现以 `HorizonCriterion` 明确组合参考高度、`Vacuum`/`Refracted` 坐标阶段和 `HorizonDiskPoint::{Center, UpperLimb, LowerLimb}`。盘面事件在每次天测求值时按 `SphericalBodyFigure` 与收敛站心距离重新计算 `asin(R/Δ)`，而不是把约 `16′` 太阳半径或月球半径写死；`astronomical_clock` 同时展示真空盘面接触和固定 `34′` 标准地平折射接触。该固定折射判据不冒充实时 SOFA 近地平折射，也不包含地形地平线。

### 18.2 晨昏蒙影和时段

- **F-TWI-001 P1 工作流** 日出/日落。
- **F-TWI-002 P1 工作流** 民用晨光始/昏影终，太阳高度 `-6°`。
- **F-TWI-003 P1 工作流** 航海晨光始/昏影终，太阳高度 `-12°`。
- **F-TWI-004 P1 工作流** 天文晨光始/昏影终，太阳高度 `-18°`。
- **F-TWI-005 P1 工作流** 任意太阳高度区间。
- **F-TWI-006 P2 工作流** 金色时段和蓝色时段，可配置定义。
- **F-TWI-007 P1 工作流** 极区无事件/连续区间分类。

### 18.3 可观测窗口

- **F-WIN-001 P1 工作流** 目标最低/最高高度。
- **F-WIN-002 P1 工作流** 太阳最高高度。
- **F-WIN-003 P1 工作流** 最小月距。
- **F-WIN-004 P1 工作流** 最大月光/照亮比例。
- **F-WIN-005 P1 工作流** 最大气团质量。
- **F-WIN-006 P1 工作流** 方位范围。
- **F-WIN-007 P1 工作流** 本地钟时间范围。
- **F-WIN-008 P1 工作流** 地平线遮挡。
- **F-WIN-009 P1 工作流** 约束交、并、差和区间合并。
- **F-WIN-010 P2 工作流** 多目标共同窗口。
- **F-WIN-011 P2 工作流** 最佳时刻和可解释评分分量。
- **F-WIN-012 P2 工作流** 视场旋转率、导星和曝光时长限制。

## 19. 月相、配置和极值事件

### 19.1 月相和季节

- **F-PHASE-001 P1 工作流** 朔。
- **F-PHASE-002 P1 工作流** 上弦。
- **F-PHASE-003 P1 工作流** 望。
- **F-PHASE-004 P1 工作流** 下弦。
- **F-PHASE-005 P1 工作流** 任意相位角。
- **F-PHASE-006 P1 工作流** 照亮比例及盈亏方向。
- **F-PHASE-007 P1 工作流** 朔望月周期、相邻月相搜索和区间统计。

当前已实现 `Events::moon_phases_in` 与 `Events::moon_phase_year`：以月球、太阳的地心视位置构造日期真黄道经度差，连续化后扫描并用 Brent 精化 $0^\circ/90^\circ/180^\circ/270^\circ$ 四个主相位。事件同时保留月球与太阳视位置、时间括区间、角残差、迭代数和历表求值数；全年接口按显式固定 UTC 偏移筛选公历年份，不把时区或 UTC 年份混入物理事件定义。

`MoonPhaseAngle` 把任意目标限定在有向整圈 `[0, 2π)`，避免与月心处无向 `[0, π]` 物理 `PhaseAngle` 混用；`Events::moon_phase_angle_in` 在闭物理时间区间内搜索该目标的每次过境，通常每朔望月一次。`MoonPhaseAngleEvent` 保留目标角、实际视黄经差、同一接收历元的日月视位置和完整数值证据。四个主月相事件复用这一实现，而不是维护第二套精化模型。

`Astrometry::lunar_illumination_at` 在同一地球接收历元分别求解月球到地球、太阳到地球的视位置链，并在观测月光离开月球的历元继续求解太阳到月球的照明光行时。结果保留三条光行时，给出地心视日月距角、有向视黄经差、月心处的物理日—月—地相位角、球形月面照亮比例，以及按 `[0, π)` / `[π, 2π)` 定义的盈/亏分支。它不含站心视差、月面地形、天平动或冲日增亮。

上述实现覆盖 F-PHASE-001 至 F-PHASE-007：`Events::synodic_months_in` 直接把相邻同一 `MoonPhaseAngleEvent` 组成 `MeasuredCycle<SynodicMonth, S>`，并可由 `CycleStatistics` 统计闭区间内完整朔望月；不另建平均月相公式。

验证结果：

- 2024 年 UTC 的 50 项 DE440 主月相已逐项对照 [USNO 分钟表](https://aa.usno.navy.mil/api/moon/phases/year?year=2024)，最大差值 39.704489 秒。
- 2024-03-25 07:00 UTC 的 DE440 月面照亮比例为 99.992998582%，物理相位角为 0.958850084°；[JPL Horizons DE441](https://ssd.jpl.nasa.gov/api/horizons.api?format=text&COMMAND=%27301%27&CENTER=%27500%40399%27&MAKE_EPHEM=%27YES%27&EPHEM_TYPE=%27OBSERVER%27&START_TIME=%272024-03-25%2007%3A00%27&STOP_TIME=%272024-03-25%2007%3A02%27&STEP_SIZE=%271%20m%27&QUANTITIES=%2710%2C24%27) 给出 99.99300% 和 0.9587°。Horizons 的 `S-T-O` 额外包含下行光路恒星光行差，因此预期与物理相位角存在数角秒差异。
- 任意 $45^\circ$ 有向月相角的 DE440 事件为 2024-03-13 14:32:28.766108145 UTC；同一毫秒的 JPL Horizons DE441 地心视黄经为[月球 38.5055006°](https://ssd.jpl.nasa.gov/api/horizons.api?format=text&COMMAND=%27301%27&CENTER=%27500%40399%27&MAKE_EPHEM=%27YES%27&EPHEM_TYPE=%27OBSERVER%27&START_TIME=%272024-03-13%2014%3A32%3A28.766%27&STOP_TIME=%272024-03-13%2014%3A33%3A28.766%27&STEP_SIZE=%271%20m%27&QUANTITIES=%2731%27)、[太阳 353.5054999°](https://ssd.jpl.nasa.gov/api/horizons.api?format=text&COMMAND=%2710%27&CENTER=%27500%40399%27&MAKE_EPHEM=%27YES%27&EPHEM_TYPE=%27OBSERVER%27&START_TIME=%272024-03-13%2014%3A32%3A28.766%27&STOP_TIME=%272024-03-13%2014%3A33%3A28.766%27&STEP_SIZE=%271%20m%27&QUANTITIES=%2731%27)，有向差为 45.0000007°。

可运行月球示例：

- `lunar_illumination`：输出一个固定 UTC 历元的照亮比例、相位角、有向视黄经差、日月视角距，以及月—地、日—地、日—月三条光行时诊断。
- `lunar_phase_angle`：搜索 2024 年 3—4 月间任意 $45^\circ$ 有向月相角，并输出事件时刻、实际角度、盈亏分支、照亮比例和数值精化证据。
- `lunar_phases_year`：按指定公历年与固定 UTC 小时偏移输出主月相及数值精化证据。

- **F-SEASON-001 P1 工作流** 春分、夏至、秋分、冬至，复用太阳地心视黄经事件链。
- **F-SEASON-002 P1 工作流** 太阳地心视黄经达到任意 $15^\circ$ 节气网格位置；视黄经采用收敛接收光行时、周年光行差和 IAU 2006 日期真黄道语义。
- **F-SEASON-003 P1 工作流** 按显式固定 UTC 偏移生成恰好 24 项、民用时间有序的公历节气年。

2024 年 UTC+08:00 的 24 项 DE440s 结果已逐项对照[香港天文台分钟表](https://www.hko.gov.hk/en/gts/astronomy/data/files/24SolarTerms_2024.xml)；全部落在公布分钟的舍入区间内，最大差值 29.753510 秒。

可运行太阳示例：

- `solar_apparent_position`：固定 UTC 时刻的太阳地心视黄经、视黄纬、日期真赤经/赤纬、距离和光行时诊断。
- `current_solar_position`：默认从设备时钟取得当前时刻，也接受调用者给出的可复现 UTC 历元；读取调用者提供的 DE BSP、IERS `finals.all`、WGS84 经度/纬度/椭球高以及气压、温度、相对湿度和波长，执行 `FixedSite → FixedObserverAt → VacuumObservedPlace → ObservedPlace`，同时输出真空/折射后高度、CIRS 赤经/赤纬、距离、太阳视直径、光行时诊断、太阳偏折处置、折射量和适用范围。它应用站心视差、有限距离太阳偏折判断、地球与站点组合光行差、IAU 2006/2000A 地球姿态、极移和 SOFA 大气折射；太阳作为偏折体本身明确不自偏折。当前仍不含 Shapiro 延迟。示例使用 `EarthAttitudeTable`，因此当前 `finals.all` 预报行即使缺少 LOD 也可运行；站点速度显式标记为 IERS 名义自转率，不冒充观测 LOD 修正。
- `solar_terms_year`：按指定公历年与固定 UTC 小时偏移输出 24 个节气及数值精化不确定度。

### 19.2 行星配置

- **F-CFG-001 P1 工作流** 两天体合。
- **F-CFG-002 P1 工作流** 外行星冲。
- **F-CFG-003 P1 工作流** 东方照和西方照。
- **F-CFG-004 P1 工作流** 内行星下合和上合。
- **F-CFG-005 P1 工作流** 最大东距和最大西距。
- **F-CFG-006 P1 工作流** 驻留与顺行/逆行转换。
- **F-CFG-007 P1 工作流** 黄经差、赤经差和真实角距三种明确判据。
- **F-CFG-008 P1 工作流** 地心与站心配置选择。
- **F-CFG-009 P1 工作流** 几何与视位置配置选择。

当前实现覆盖 F-CFG-001–009。`RelativeBodyQuery` 固定目标减参考天体的有向次序和 `Geometric`/`Apparent` 语义；`ConfigurationQuery` 再固定合、冲、东方照、西方照及日期真黄经差或真赤经差判据。太阳参考合事件由同一观测原点到目标和太阳的实际距离分类为内合/外合，不由天体名称硬编码。`greatest_elongations_in` 对真实球面角距求极大并按黄经差返回东/西分支；`stations_in` 对连续化日期真黄经的时间导数求根并保留顺行/逆行转换两侧状态。所有工作流同时提供地心入口；完整 EOP 上下文另提供固定站点入口，站心路径复用 `FixedObserverAt` 的光行时、视差、光行差和地球姿态链。

### 19.3 距离、角距和交点

- **F-EXT-001 P1 工作流** 天体间角距离最小/最大。
- **F-EXT-002 P1 工作流** 物理距离最近/最远。
- **F-EXT-003 P1 工作流** 月球近地点/远地点。
- **F-EXT-004 P1 工作流** 行星近日点/远日点。
- **F-EXT-005 P1 工作流** 升交点/降交点通过黄道。
- **F-EXT-006 P1 工作流** 赤道穿越。
- **F-EXT-007 P1 工作流** 黄纬/赤纬极值。
- **F-EXT-008 P2 工作流** 轨道平面交点和近节点时刻。
- **F-EXT-009 P1 工作流** 角直径、亮度、相位角和照亮面积极值。

当前实现覆盖 F-EXT-001–007。`AngularSeparationExtremumQuery` 搜索真实球面角距而不是经度差代理；`DistanceExtremumQuery` 搜索同时几何天体间距离，可直接表达月球近/远地点和行星近日/远日点；`CoordinateCrossingQuery` 与 `CoordinateExtremumQuery` 明确选择日期真黄纬或真赤纬，并保留交越方向或极值种类。F-EXT-008 的任意轨道平面节点和 F-EXT-009 的角直径、亮度、相位角及照亮面积复合极值尚未实现。

验证结果：本地 DE440 在 2024 全年得到 6 次水星合、7 次最大距、7 次驻留和 13 次月球近地点；2024-03-24 最大东距为 $18.701601878^\circ$、UTC 22:34:04.866，与 [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/) 在 22:34 UTC 给出的 $18.7016^\circ$ 一致。契约测试还以北京固定站点验证站心与地心角距极值的时刻和角度均不同，防止固定站点入口退化为地心计算。可运行示例为 `planetary_events`。

### 19.4 天文年与月周期

- **F-PERIOD-001 P1 工作流** 由相邻事件测得的天文周期结果保留周期种类、首尾物理事件、实际 `Duration`、时间尺度、模型/历表标识和数值搜索证据。
- **F-PERIOD-002 P1 工作流** 回归年与同名分点年分开：回归年是带求值历元和平均太阳/分点模型的局部平均周期，分点年测量相邻同类实际分点事件。
- **F-PERIOD-003 P2 工作流** 恒星年以固定惯性参考方向和日心地球状态的一整圈为判据。
- **F-PERIOD-004 P2 工作流** 近点年以相邻地球近日点事件为判据。
- **F-PERIOD-005 P2 工作流** 交点年以太阳相对同一月球轨道交点的一整圈为判据。
- **F-PERIOD-006 P1 工作流** 朔望月由同一有向月相的相邻事件测量，复用 `Events::moon_phase_angle_in`，不另建近似月相链。
- **F-PERIOD-007 P2 工作流** 恒星月和回归月分别使用固定惯性黄道与日期平均分点黄道的一整圈判据。
- **F-PERIOD-008 P2 工作流** 近点月使用相邻月球近地点事件，交点月使用同向同一交点穿越事件。
- **F-PERIOD-009 P1 工作流** 对闭区间内完整周期计算样本数、最小值、最大值、算术平均值和标准差；被区间截断的周期不得混入统计。
- **F-PERIOD-010 P1 内核** 文献平均周期只能作为带来源、历元和适用范围的参考值或搜索步长，不得伪装成精确物理时长或替代事件求解。

当前 `event::period` 已实现统一的 `MeasuredCycle<K, S>`、强类型首尾事件、`CycleEvidence`、内核清单与参考轴溯源以及 `CycleStatistics`。`Events` 公开分点年、恒星年、近点年、交点年、朔望月、恒星月、回归月、近点月和交点月的闭区间工作流；`ModeledCycle<TropicalYear, S>` 另行提供带求值历元与 J500.0—J3500.0 推荐范围的 Meeus 平均太阳黄经导数模型，不把它冒充相邻分点事件间隔。近点年在地心日距的候选极小值中解析月球尺度子结构，交点年使用日期平均黄道上的瞬时月球密切轨道交点。

验证结果：`tests/period_contracts.rs` 以本地 DE440 在 2021—2025 年区间运行全部九种事件测量工作流，并检查周期量级、首尾顺序、历表溯源和统计不变量；`astronomical_cycles` 示例输出分点年与朔望月的样本统计。长期 BSP 工作流应使用 `--release`：同一三年示例在当前工作站的发布构建纯运行耗时 0.33 秒；调试构建因未优化的多次光行时与框架链耗时 23.75 秒。

## 20. 掩星、凌日与食

### 20.1 通用遮掩几何

- **F-OCC-001 P2 内核** 点源被有限圆盘遮掩。
- **F-OCC-002 P2 内核** 两有限圆盘外切、内切和重叠面积。
- **F-OCC-003 P2 内核** 球体/椭球体视轮廓。
- **F-OCC-004 P2 工作流** 掩星初筛和最近角距离。
- **F-OCC-005 P2 工作流** 第 1、2、3、4 接触。
- **F-OCC-006 P2 工作流** 全掩、环掩、偏掩、凌和相切分类。
- **F-OCC-007 P2 工作流** 食甚/最近时刻、食分和遮掩面积。
- **F-OCC-008 P2 工作流** 接触位置角和目标高度。

### 20.2 日食

- **F-SOL-001 P2 工作流** 朔附近地心日食初筛。
- **F-SOL-002 P2 工作流** 偏食、全食、环食和全环食分类。
- **F-SOL-003 P2 内核** 贝塞尔基本平面。
- **F-SOL-004 P2 工作流** 贝塞尔根数 `x,y,d,μ,l1,l2` 及导数。
- **F-SOL-005 P2 工作流** 全局 P1/P2/U1/U2/U3/U4/P3/P4 相关接触语义。
- **F-SOL-006 P2 工作流** 中心线、北/南界和路径宽度。
- **F-SOL-007 P2 工作流** 最大食地点和时刻。
- **F-SOL-008 P2 工作流** 地方初亏、食既、食甚、生光和复圆。
- **F-SOL-009 P2 工作流** 地方食分、遮掩比例、太阳高度和方位。
- **F-SOL-010 P2 工作流** 日出/日落带食和不可见接触分类。
- **F-SOL-011 P2 数据** 地球椭球、Delta T、EOP 和月面半径模型。
- **F-SOL-012 P2 数据** 月缘地形对全食/环食界线和贝利珠的可选修正。
- **F-SOL-013 P2 工作流** 路径 GeoJSON/采样输出适配。

当前 `event::solar_eclipse` 已实现地方日食垂直切片：`Events::local_solar_eclipses_in` 以地心视朔作候选种子，在固定站点真空视位置上直接最大化食分，并分别求解外切 C1/C4 与全食或环食的内切 C2/C3。工作流既接受完整 `EarthOrientationTable` 观测姿态，也接受显式 `PredictedEarthOrientation` 场景；后者由版本化 `DeltaTModel` 直接给出 `TT−UT1`，并用具名极移/天极偏差预测或假设产生方向姿态，站点速度明确使用 IERS 名义自转率而不伪造 LOD。`LocalSolarEclipse<S>` 保留偏食/环食/全食分类、完整接触、食甚、食分、圆盘重叠遮掩比例、太阳高度/方位、接触位置角、阶段持续时间、球形日月半径模型、完整观测/预测姿态 provenance、历表 provenance 和数值证据；太阳视盘越过天文地平线的可见性可逐事件查询。默认解析历表用于近似和初筛；`tests/local_solar_eclipse_contracts.rs` 同时覆盖 2024 IERS C04 观测路径与 2035 `Delta T` 预测路径，并以 DE440、IERS C04 和 USNO 2024-04-08 Dallas 地方资料验证时刻、位置角、食分和持续时间。

当前 `event::global_solar_eclipse` 的全球阴影锥分类不依赖贝塞尔近似：`Events::global_solar_eclipses_in` 在视朔附近最小化日月阴影轴到地心的距离，以调用者选择的旋转参考椭球和球形日月半径建立精确公切锥。结果保留带南北符号的 `SolarEclipseGamma`、全球食甚时刻、阴影轴距离、半影/本影/伪本影与椭球的数值交会、中心轴掠入/掠出区间、非中心全食/环食分类，以及锥顶穿越近侧地表的全环食转换时刻。独立的 `Events::solar_eclipse_besselian_elements_at` 已实现指定历元的 F-SOL-003—004 根数和 60 秒 TT 对称导数；`BesselianElementsOptions` 要求显式选择 `BesselianLimbModel`，可使用单一物理月球球面，或 NASA Five Millennium Canon 的 696000 km 太阳半径、`k1=0.272488`、`k2=0.272281` 与默认零 `Δb/Δl`。非零 `Δb/Δl` 会实际修正月球日期真黄纬/真黄经后再参与阴影轴计算，不是未应用的元数据。`Events::solar_eclipse_besselian_polynomial` 以五个等间隔视位置样本拟合六小时发布表：`x/y` 三次、`d/l1/l2` 二次、`μ` 一次，保留解析导数、闭有效区间、样本最大残差、模型和历表来源，区间外拒绝外推。`μ` 采用 TT 历书时角，旋转地球应用仍须由调用者另给 `ΔT=TT−UT1`。`tests/besselian_elements_contracts.rs` 以非零 `Δb/Δl` 行为契约及 2024-04-08 NASA 表和 DE440 验证半径常数、全部系数与导数；全球四分类及非中心食由 `tests/global_solar_eclipse_contracts.rs` 覆盖。因此 F-SOL-001—004 已实现，F-SOL-007 的全球食甚时刻部分已实现；其地理地点与 F-SOL-006 的路径能力由下述 `solar_eclipse_path` 工作流实现。

`Events::solar_eclipse_path` 现已实现 F-SOL-006 和 F-SOL-007 的地理部分：它把同一历表、同一参考椭球的 `GlobalSolarEclipse<S>` 与六小时 `BesselianElementsPolynomial<S>` 组合，并要求调用者提供在多项式参考历元解析的强类型 `DeltaT<S>`。每个 `GlobalSolarEclipsePathPoint<S>` 保存中心线、运动核心影包络的北/南界、同一时刻两界的椭球反解测地跨度 `boundary_geodesic_span`、按贝塞尔路径公式投影到垂直于中心线运动方向的横向 `path_width`、固定中心线站点的 C2/C3 与中心阶段持续时间、环食/全食性质，以及无折射太阳高度和方位；路径默认以两分钟采样，另保留阴影轴掠入/掠出的完整时间区间。北/南界由“核心影锥落在地表”与“固定地表点接触残差对时间的一阶导数为零”联立得到，是运动阴影的路径包络，不是瞬时影斑的纬度极值。日出/日落附近可能只有一条包络分支，这些单边时刻保留在路径时间区间中，但不伪造完整双边截面。`tests/solar_eclipse_path_contracts.rs` 以解析历表验证结构不变量，并以 DE440、IERS C04、NASA 路径表验证食甚地点、边界测地跨度、横向路径宽度、中心持续时间和太阳高度/方位。F-SOL-013 的强类型采样部分已实现，GeoJSON 输出适配仍未实现；月缘地形修正、全局 P/U 接触语义及地方日出/日落带食分类仍分别属于 F-SOL-012、F-SOL-005 和 F-SOL-010 的未完成部分。


### 20.3 月食

- **F-LUN-001 P2 工作流** 半影、偏食和全食分类。
- **F-LUN-002 P2 工作流** P1、U1、U2、最大、U3、U4、P4 时刻。
- **F-LUN-003 P2 工作流** 半影食分和本影食分。
- **F-LUN-004 P2 工作流** 各阶段持续时间。
- **F-LUN-005 P2 工作流** 月面接触位置角。
- **F-LUN-006 P2 数据** 地球本影/半影扩大模型和大气经验参数。
- **F-LUN-007 P2 工作流** 地方可见性、月球高度和晨昏背景。

当前 `event::lunar_eclipse` 已实现 F-LUN-001—007。`Events::global_lunar_eclipses_in` 以地心视满月作种子，在日期真赤道轴上最小化月心到反日地影轴的角距；`LunarShadowGeometry<S>` 由同一接收历元的日月视位置、调用者的 `Earth` 赤道半径和 IAU 球形日月模型计算半影/本影角半径、月面轴距、半影/本影食分与接触位置角。`LunarShadowConvention` 显式区分无大气几何、NASA Five Millennium Catalog 使用的 Danjon `1.01` 有效地球视差约定，以及 Chauvenet `0.998340` 视差后统一 `1.02` 阴影扩大约定，不把经验边界伪装成唯一物理边界。结果按半影/偏食/全食分类，保留 P1、U1、U2、食甚、U3、U4、P4、嵌套阶段区间与持续时间、数值括根/极值证据、模型和历表 provenance。

`Events::local_lunar_eclipse_visibility` 只在完整 `EarthOrientationTable` 上公开：它以同一固定站点和选定的中心/上下缘、真空/折射地平判据求解 P1—P4 内月出月落，把全球半影、偏食和全食区间分别与地平线上区间求交，并在每个接触和食甚保留月球站心位置、月球高度、低空标志、太阳真空高度及白昼/民用曙暮光/航海曙暮光/天文曙暮光/夜间背景。`tests/lunar_eclipse_contracts.rs` 覆盖三种分类、Dallas 月落截断和低空提示，并以 DE440 对照 NASA 2022-11-08 的接触时刻、食分和位置角；`examples/analytic_lunar_eclipses.rs` 提供解析历表和可选 BSP 的全年全球—地方工作流。月食实际亮度衰减、云量和地形地平线不属于 F-LUN-001—007，仍分别由光度、天气输入和地形模型承担。

### 20.4 掩星和凌日

- **F-STAROCC-001 P2 工作流** 月掩恒星/行星的地心初筛。
- **F-STAROCC-002 P2 工作流** 地方掩始/掩终、位置角和暗/亮边。
- **F-STAROCC-003 P2 数据** 月缘 Watts/Kaguya/LOLA 类轮廓适配。
- **F-STAROCC-004 P2 工作流** 小行星掩星中心线、路径和不确定带。
- **F-TRANSIT-001 P2 工作流** 水星/金星凌日四接触和最大阶段。
- **F-TRANSIT-002 P2 工作流** 行星卫星凌、掩、食和影凌。
- **F-TRANSIT-003 P2 工作流** 任意前景/背景目标的通用凌事件。

### 20.5 周期与系列

- **F-CYCLE-001 P2 数据** 日食/月食沙罗系列编号。
- **F-CYCLE-002 P2 工作流** 相邻沙罗成员搜索。
- **F-CYCLE-003 P2 数据** Inex、semester 等分类周期作为检索元数据。
- **F-CYCLE-004 P2 工作流** 周期只提供搜索种子，精确事件由历表重新求解。

## 21. 人造卫星事件

- **F-SAT-001 P2 工作流** 卫星升起、最高点和落下。
- **F-SAT-002 P2 工作流** 地影进入/离开和日照状态。
- **F-SAT-003 P2 工作流** 可见过境窗口：太阳高度、卫星日照和目标高度组合。
- **F-SAT-004 P2 工作流** 地面轨迹、星下点和覆盖圆。
- **F-SAT-005 P2 工作流** 多站可见性和链路几何。
- **F-SAT-006 P2 工作流** 卫星与太阳/月球/恒星角距阈值。
- **F-SAT-007 P2 数据** TLE 历元和年龄适用范围检查。

## 22. 星表空间检索与天区

- **F-SKY-001 P1 内核** 圆锥检索。
- **F-SKY-002 P1 内核** 最近邻和 k 近邻。
- **F-SKY-003 P1 工作流** 传播到共同历元后交叉匹配。
- **F-SKY-004 P1 工作流** 自行和协方差扩张搜索半径。
- **F-SKY-005 P1 内核** 角距排序和歧义候选。
- **F-SKY-006 P1 内核** 马氏距离匹配。
- **F-HPX-001 P2 适配** HEALPix ring/nested 编码。
- **F-HPX-002 P2 适配** 像素中心、边界、邻居和父子层级。
- **F-HPX-003 P2 适配** 圆锥到像素覆盖。
- **F-MOC-001 P2 适配** MOC 区域读取、合并和包含。
- **F-CONSTELL-001 P2 数据** IAU 星座边界和星座查询。
- **F-NAME-001 P2 数据** 离线天体别名解析为 hyastro 天体类型。

## 23. WCS、视场与成像

- **F-WCS-001 P2 适配** FITS WCS 基础关键字解析。
- **F-WCS-002 P2 内核** TAN/gnomonic 天球投影。
- **F-WCS-003 P2 适配** 常见 zenithal/cylindrical 投影。
- **F-WCS-004 P2 适配** SIP 等畸变模型。
- **F-WCS-005 P2 工作流** 像素↔天空方向。
- **F-WCS-006 P2 工作流** 切平面、板比例、旋转和视场角。
- **F-WCS-007 P2 工作流** 星表投影到焦平面。
- **F-WCS-008 P2 工作流** 视场覆盖和天区多边形。
- **F-WCS-009 P2 工作流** 曝光期间场旋和拖线估计。

## 24. 不确定度

- **F-UNC-001 P1 内核** 标准差、相关矩阵和协方差。
- **F-UNC-002 P1 内核** 单位化协方差参数顺序。
- **F-UNC-003 P1 内核** 解析 Jacobian 传播。
- **F-UNC-004 P1 内核** 数值 Jacobian 传播。
- **F-UNC-005 P2 内核** Monte Carlo 传播。
- **F-UNC-006 P1 工作流** 星表历元传播不确定度。
- **F-UNC-007 P1 工作流** 参考系/EOP 不确定度。
- **F-UNC-008 P2 工作流** 事件时刻、食路径和掩星路径不确定度。

当前实现了 F-UNC-001 的强类型标准不确定度、有限对称半正定 `CorrelationMatrix<N>`，以及六参数星表完整协方差；`StandardUncertainty<Q>` 保留物理量和规范单位并拒绝负值。F-UNC-002、F-UNC-004 和 F-UNC-006 的首个纵向切片固定采用 $\alpha*$、$\delta$、$\varpi$、$\mu_{\alpha *}$、$\mu_\delta$、$v_r$ 顺序，分别使用 rad、rad、rad、rad/s、rad/s、m/s；围绕 SOFA `starpm` 的局部切平面五点数值 Jacobian 执行 $J C J^\mathsf{T}$，同时返回 Jacobian、传播后的类型化标准不确定度和相关矩阵。

F-UNC-007 的 EOP 入口解析 IERS C04 误差列和 `finals2000A` Bulletin A 误差列。区间端点采用相关性未知时的线性上界 $(1-t)\sigma_0+t\sigma_1$，结果以 `UncertaintyOrigin` 明示来源或传播规则；任一端缺值时不伪造结果。通用解析 Jacobian、Monte Carlo、Gaia 数据适配，以及 EOP 经姿态矩阵继续传播到天球方向仍未实现。

## 25. 数据格式、序列化与互操作

- **F-IO-001 P0 内核** 稳定 `Display` 和调试输出分开。
- **F-IO-002 P1 适配** 可选 Serde。
- **F-IO-003 P1 数据** 带 schema 版本的无损时间序列化。
- **F-IO-004 P1 数据** 带单位、参考系、原点和历元的坐标序列化。
- **F-IO-005 P1 适配** CSV/ECSV。
- **F-IO-006 P2 适配** VOTable。
- **F-IO-007 P2 适配** FITS 表和图像头。
- **F-IO-008 P2 适配** Parquet/Arrow。
- **F-IO-009 P0 适配** IERS 文本数据。
- **F-IO-010 P0 适配** DAF/SPK/BSP。
- **F-IO-011 P2 适配** SPICE PCK/FK/CK/LSK/SCLK。
- **F-IO-012 P2 适配** MPC/TLE/OMM/IONEX。
- **F-IO-013 P1 适配** Python 窄绑定。
- **F-IO-014 P1 适配** WASM 纯 Rust 构建。
- **F-IO-015 P2 适配** C ABI 窄绑定，禁止暴露 Rust 布局。

## 26. 上下文、缓存和批处理

- **F-CTX-001 P0 工作流** `TimeContext` 持有闰秒、EOP 和时间模型。
- **F-CTX-002 P0 工作流** `Frames` 持有参考系模型和动态架注册表。
- **F-CTX-003 P0 工作流** `Astrometry` 持有历表、地球、偏折和大气策略。
- **F-CTX-004 P0 工作流** 所有上下文不可变或内部同步，对调用者无隐藏全局状态。
- **F-CTX-005 P0 工作流** 严格、标准、快速精度策略。
- **F-CTX-006 P0 工作流** 明确迭代上限、容差和外推策略。
- **F-CACHE-001 P1 内核** 历表记录缓存。
- **F-CACHE-002 P1 内核** 同一时刻地球定向中间量缓存。
- **F-CACHE-003 P1 内核** 缓存键包含数据和模型版本。
- **F-CACHE-004 P1 内核** 有界容量、显式清理和命中统计。
- **F-BATCH-001 P1 工作流** 批量时间转换。
- **F-BATCH-002 P1 工作流** 批量参考系转换。
- **F-BATCH-003 P1 工作流** 批量星表传播和观测位置。
- **F-BATCH-004 P1 工作流** 调用者提供输出缓冲。
- **F-BATCH-005 P1 适配** 可选 Rayon 并行且输出顺序确定。

## 27. 错误、诊断和安全

- **F-ERR-001 P0 内核** 范围错误。
- **F-ERR-002 P0 内核** 不存在/歧义时间错误。
- **F-ERR-003 P0 数据** 数据缺失、过期和覆盖外错误。
- **F-ERR-004 P0 内核** 参考系路径/原点不匹配错误。
- **F-ERR-005 P0 适配** 历表目标、段类型和后端错误。
- **F-ERR-006 P0 内核** 退化几何错误。
- **F-ERR-007 P0 内核** 不收敛和无括区间错误。
- **F-ERR-008 P0 适配** 文件格式、偏移和资源上限错误。
- **F-ERR-009 P0 适配** 上游状态码和 FFI 错误映射。
- **F-ERR-010 P0 内核** 精度无法保证错误。
- **F-ERR-011 P0 内核** 错误包含字段、时刻、目标、覆盖和恢复信息。
- **F-SAFE-001 P0 适配** 外部整数和偏移 checked arithmetic。
- **F-SAFE-002 P0 适配** 解析内存/记录/字符串上限。
- **F-SAFE-003 P0 适配** `unsafe` 隔离和不变量说明。
- **F-SAFE-004 P0 适配** FFI 全局状态同步。
- **F-SAFE-005 P0 内核** 核心计算不因普通坏输入 panic。
- **F-SAFE-006 P1 适配** BSP、FITS、VOTable、CSV、IERS 解析 fuzz。

## 28. 性能、平台和发布能力

- **F-PLAT-001 P0 内核** 稳定 Rust 和固定 MSRV。
- **F-PLAT-002 P0 内核** Linux、macOS、Windows 64 位。
- **F-PLAT-003 P1 内核** `math`/纯时间表示 `no_std`。
- **F-PLAT-004 P1 内核** 可选 `alloc`。
- **F-PLAT-005 P1 适配** WASM 不依赖 C FFI。
- **F-PERF-001 P0 内核** 基础量、向量和旋转零分配。
- **F-PERF-002 P0 适配** 已加载 SPK 单点查询零分配。
- **F-PERF-003 P1 适配** 大文件内存映射/流式读取。
- **F-PERF-004 P1 内核** 时间、地球定向、SPK、星表和事件基准。
- **F-PERF-005 P1 内核** 分配计数回归。
- **F-FEAT-001 P0 工程** Cargo feature 加法性。
- **F-FEAT-002 P0 工程** 默认构建纯 Rust、离线、最小依赖。
- **F-FEAT-003 P0 工程** `serde`、`rayon`、`anise`、`logging`、`catalog-csv`、`text-parsing`、`geodesy`、`vsop87`、`compression`、`integrity`、`fits`、`votable`、`parquet`、`healpix`、`moc`、`sgp4`、`timezone` 独立启用。
- **F-REL-001 P0 工程** SemVer、序列化 schema 版本和模型/数据变更记录。
- **F-REL-002 P0 工程** 依赖许可证、MSRV、平台和上游版本审计。

## 29. 测试能力

- **F-TEST-001 P0 测试** 角度、单位、向量、旋转不变量单元测试。
- **F-TEST-002 P0 测试** 历法、闰秒、两段式时间和尺度边界测试。
- **F-TEST-003 P0 测试** SOFA/ERFA 官方验证向量。
- **F-TEST-004 P0 测试** IERS 地球定向样例。
- **F-TEST-005 P0 测试** CSPICE/JPL SPK 差分样例。
- **F-TEST-006 P1 测试** Gaia DR3 官方字段和固定小样本。
- **F-TEST-007 P1 测试** NOVAS/SOFA/ERFA 独立差分路径。
- **F-TEST-008 P0 测试** 旋转、时间和坐标往返性质测试。
- **F-TEST-009 P1 测试** 极点、对跖、跨零、负年份、闰秒和覆盖端点生成测试。
- **F-TEST-010 P1 测试** 真实 EOP、DE BSP、Gaia、站点端到端观测位置。
- **F-TEST-011 P1 测试** 升落、月相和配置事件独立参考结果。
- **F-TEST-012 P2 测试** 日月食、掩星接触和路径独立参考结果。
- **F-TEST-013 P0 测试** 每个缺陷的最小回归样例。
- **F-TEST-014 P1 测试** 解析 fuzz 和 FFI 边界检查。
- **F-TEST-015 P1 测试** 热路径吞吐、延迟和分配基准。

## 30. 文档与开发者体验

- **F-DOC-001 P0 文档** 领域词汇表。
- **F-DOC-002 P0 文档** 每个公开类型的单位、范围、尺度、参考系和原点。
- **F-DOC-003 P0 文档** 每个算法的标准、模型版本和有效期。
- **F-DOC-004 P0 文档** 每个后端的数据、许可证、覆盖和精度说明。
- **F-DOC-005 P0 文档** 时间转换、参考系转换、星表观测和历表查询教程。
- **F-DOC-006 P1 文档** 升落/月相/观测窗口教程。
- **F-DOC-007 P2 文档** 食、掩星、小天体和卫星教程。
- **F-DOC-008 P0 文档** 可运行 rustdoc 示例。
- **F-DOC-009 P0 文档** 错误恢复、严格模式和数据更新指南。
- **F-DOC-010 P0 文档** 版本升级和结果变化说明。

## 31. 完整性边界

本目录覆盖基础天文、天体测量、时间、参考系、地球定向、星表、历表、观测者、大气、轨道和球面事件。以下领域只提供接缝，不在本库重复建设完整产品：

- 航天任务设计、最优控制、定轨滤波和导航解算由外部动力学系统承担；hyastro 提供强类型状态、时间、参考系、历表和事件接缝。
- 在线 TAP/ADQL 星表服务、自动数据下载和凭证管理由应用层承担；hyastro 提供离线解析和数据源接口。
- 望远镜硬件控制、ASCOM/INDI 设备协议和调度执行由观测应用承担；hyastro 提供指向、窗口、场旋和大气结果。
- 图形渲染和交互星图由 UI/渲染库承担；hyastro 提供坐标、投影、天区和事件数据。
- 通用统计推断、MCMC 和大型线性代数由专门数值库承担；hyastro 提供领域 Jacobian、协方差和可采样工作流。

跨越这些边界时必须保留时间尺度、单位、参考系、原点、历元、数据版本和质量，不允许在适配层退化为无语义裸数组。
