# 分层架构与扩展约定

## 边界

| 层 | 职责 | 允许的项目依赖 |
| --- | --- | --- |
| ui | 设置页、悬浮卡片、用户输入、绘制 | application、domain、interfaces、assets |
| application | 监测任务生命周期、执行用例、协调服务 | domain、interfaces |
| domain | 资源模型、设置校验、阈值和清理策略 | 本层 |
| interfaces | 采样、存储、桌面集成、清理等能力契约 | domain |
| infrastructure | Windows API 与文件存储的实际实现 | domain、interfaces、assets、本层 |
| main.rs | 解析入口参数、选择并装配实现、启动 UI | 所有层 |

这里的接口层是内部能力契约，当前没有 HTTP 服务。`ResourceSampler`、`SettingsStore`、`DesktopSession`、`MemoryCleaner` 是实际使用的接口，不为空目录或占位模块。

UI 不导入 infrastructure，不直接读取设置文件或调用 Win32。后台采样服务通过工厂在采样线程内创建适配器，Windows 句柄始终由该线程创建、使用和释放，避免为了跨线程传递而添加不安全的 Send 实现。

窗口、托盘和提示框由 `WindowsDesktop` 实现 `DesktopSession`；UI 通过接口发出意图。唤醒界面通过 `Wake` 回调传入，基础设施不依赖 egui。主入口与其bootstrap支持模块构成组合根。

宠物绘制、局部头部/眼睛网格动画与快捷面板属于 UI；`DesktopGeometry`、宠物选项和面板边界计算属于领域层。全屏鼠标位置、原生拖放通知和工具窗口样式通过 `DesktopSession` 提供。普通悬浮窗与快捷面板同时设置框架的任务栏选项和 Win32 工具窗口样式，设置窗口保留普通应用样式。Windows 样式变化拦截保持任务栏策略，不改变置顶或鼠标穿透设置。

## 层内按职责组织

`assets.rs` 是框架中立的静态 RGBA 目录，由 Rust 构建生成，不依赖 UI、Windows 或文件存储。UI 使用 egui 按需缓存；原生菜单使用 GDI 位图，同享像素而不跨层调用。`scripts/visual_assets.rs` 在构建期读取原图与切图清单，生成尺寸、质量、预算及切片，运行时不做图像解码；开发测试直接覆盖同一切图逻辑。未实现功能的素材不链接进应用。

- `ui/monitor.rs` 保留主界面入口与生命周期协调；主入口使用方式保持兼容。
- `ui/common/` 放各窗口复用的主题、图标、控件及清理档位/确认组件，不依赖具体设置页。宠物快捷面板与清理页复用同一确认逻辑。
- `ui/monitoring/` 放资源卡片、操作栏、详情、趋势与卡片表面；`ui/companion/` 放宠物、动作与快捷入口。
- `ui/work_tools/work_timer/` 放工作计时的页面、圆环和桌面提醒；`ui/monitor/work_timer.rs` 接主循环的定时刷新与入口意图，避免把工具堆进主监视器文件。
- `ui/settings/mod.rs` 管理设置状态、视口与页面路由；`pages/` 管理监测项、外观、设备、通用与清理页面；`shell/` 管理导航、自绘标题栏与缩放。设备和通用页不再塞进路由模块。
- `interfaces/` 按 monitoring、desktop、storage、cleanup 四类能力定义契约，`mod.rs` 仅汇总对外接口与共享唤醒类型；现有调用方不依赖契约所在文件。
- `infrastructure/windows/desktop/` 管理窗口、托盘、菜单、原生拖动、贴边和提示；`monitoring/` 管理采样、GPU、PDH 与进程；`memory/` 管理清理 API 与提升辅助进程。`native.rs` 保留跨能力使用的原生辅助，`diagnostics.rs` 保留只读诊断入口。
- application、domain 和文件存储目前规模较小，继续按职责文件组织；`domain/work_timer.rs` 定义计时规则，`application/work_timer.rs` 管理单调时间和一次提醒。录音、截图等未实现能力不预建空目录，新能力形成真实模块后再按功能归组。

模块路径随职责调整，调用方使用实际模块路径，不以旧平铺路径别名遮蔽新组织。组内实现尽量私有；主入口装配所需的能力保留公开入口。

## 新功能如何加入

系统字体由 Windows `fonts` 模块加载，接口以 `FontSource` 传递中立的名称与借用/自有字节；UI `common/fonts` 只配置 egui 字体族，主监测入口不负责字体 I/O。Windows 优先只读映射，OnceLock 保留一次成功字体的 backing 至进程退出；UI 使用静态借用，不把字节转为 Vec。映射失败才读取并保留一份缓冲，亦按静态借用交给框架。跨层字体/中文回归在 `src/integration_tests`，UI 仅在测试编译时重导出测试入口，不放宽生产层间边界。

自绘设置窗、开关、数值编辑和分类页签属于 UI；窗口按钮通过 egui 视口命令交给宿主执行，不在 UI 调 Win32。`DesktopSession::configure_auxiliary` 在辅助窗口创建后调用，Windows 的 `settings_surface` 处理设置窗的原生标题/八方向缩放命中，按 DPI 换算且保留按钮和内容客户区；仅附着当前进程的设置窗。`surface` 共享卡片与预览的尺寸/表面规则，预览消费已有采样，不启动独立采样服务。只读诊断分别暴露原始样式位和实际非客户区边距，避免把 Windows 保留的样式位当成可见系统框。

1. 在 domain 定义输入、输出、状态与规则。
2. 需要新系统能力时，在 interfaces 定义契约。
3. 在 infrastructure 实现 Windows 或持久化适配器。
4. 在 application 增加执行用例，处理并发、取消、失败与结果。
5. 在 ui 加入入口与结果呈现，通过服务调用。
6. 在 main.rs 装配实现，补回归测试和说明。

小功能可以扩展既有模块，不需要每次增加一整套目录。计时、录音、截图、待办形成实现时，按能力加入对应层；页面放在对应功能目录，共用图标与控件留在 common，禁止所有工具堆进 monitor.rs 或 settings/mod.rs。当前保持单 Cargo 包，避免仅为形式拆成多个 crate；模块已经建立依赖边界，后续有独立发布需求时再拆包。

工作计时复用主 UI 更新，不创建采样线程；运行时按下一秒或截止时刻请求刷新，暂停/结束/空闲不请求计时刷新。配置仅保存方案与提醒，运行会话不持久化。提醒卡片为非激活工具窗口，通过已有 DesktopSession 纳入本进程样式管理；原生适配器与只读诊断仍不进入 UI。采样配置命令使用 Box 保持控制枚举小尺寸，只在配置变化时分配。

## 兼容性与验证

分层螺旋的状态与工作区镜像布局位于 `domain/companion/`；UI 位于 `ui/companion/flyout/`，分为协调、渲染、功能组件和回归。`DesktopSession::quick_regions` 将圆形/圆角组件交给 Windows 适配器形成原生窗口区域并集；句柄由 RAII 和系统所有权管理，转换使用实际窗口 DPI，检查当前 PID。动画250ms内保持完整绘制包络，不逐帧改变裁剪，稳定后恢复离散组件，透明空隙此时排除在命中之外。全客户区DWM透明合成解决本机实际宽黑圈，不能替代所有显卡/DPI验证。试用入口仅在设置打开时作为普通辅助窗口存在，复用真实渲染及服务；日常入口保持工具窗口。活动入口每最多50ms检查光标，宠物可操作状态每200ms读取原生光标；动画约16ms刷新，关闭入口后没有50ms检查，无新增采样线程。

`ui/common/artwork::draw_content` 缓存静态 RGBA 的主体 UV 边界，并等比绘制材质缩略图；素材仍由同一构建管线处理，不引入运行时图片编解码依赖。

诊断报告按 `domain/report`、`interfaces/report`、`application/report`、Windows `report`、storage `reports` 和 UI `work_tools/diagnostic_report` 组织。只有显式生成/保存才创建短命工作线程，单任务限制与取消由应用层管理；采样器/句柄在工作线程内创建，结束后释放并回收线程。领域 `SafePath` 在构造时脱敏，文档只有脱敏路径；复制与保存共用冻结 Markdown，不重新采集。UI 分入口、固定操作、概览及完整只读预览；设置路由增加报告页，螺旋返回打开意图，不隐式生成。设置集成测试位于 `settings/tests`，不为测试扩大生产模块权限。开发探针 `--report-probe` 使用同一采集与渲染链路。

目录重构保留原配置位置、窗口标题和单实例标识。监测、清理、桌面集成和持久化均通过真实接口装配；清理辅助进程使用新版内部命令与具备进程来源校验的命名管道。公开只读诊断入口 `--probe`、`--inspect` 保留。

`tests/layering.rs` 在测试时检查层间禁止依赖，防止新功能重新把文件存储或 Windows API 放回 UI。测试还覆盖后台采样线程生命周期、真实 egui 点击、边界约束、清理并发与失败、命名管道传输。当前验证结果见 [本轮验证](本轮验证.md)。系统清理效果须与界面和结构验证分别判定。

用户追加全状态100 MB硬门禁，适用于所有窗口及自身辅助进程总量。缓存、缓冲、工作任务按有界容量和生命周期设计，新增功能不得规避峰值检查；当前实测失败，尚未做完整分配归因和架构优化。口径、场景与证据见 [内存预算与验收](内存预算与验收.md)。

`bootstrap/resource_probe` 为入口装配的开发观察器，显式参数才安装；组合根可同时读取框架纹理元数据、当前进程内存与有界证据文件。默认产品不安装探针，没有该文件句柄/回调/日志缓冲。它不属于UI/领域能力，也未放宽原层间禁止依赖。字体图集上限及根/即时子视口适配仍在UI框架层，驻留分类工具仅位于scripts；两者都不新增后台采样线程。取证细节见《内存分配取证》。
