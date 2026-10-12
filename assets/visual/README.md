# 视觉资产与切图

生成日期：2026-10-12。使用用户授权的内置 image_gen。原图、完整提示词、版本和切图清单均保存在项目内。当前已接入设置、悬浮操作栏、托盘、原生菜单、工作计时及分层螺旋；便签等仍待实现，素材存在不表示功能完成。

## 风格与当前取舍

烟蓝磨砂玻璃、珍珠白轮廓、香槟选中态，资源告警仍用绿/黄/红。设置使用静态生成材质与半透明容器，**没有实现读取桌面后的实时毛玻璃模糊**。装饰图像不增加后台采样。控制图标为干净轮廓，圆形玻璃底座由界面绘制；图标不使用 emoji 或文字符号。

2026-10-12 用户指出首次实机界面仅换图标和背景，与概念图相差过大。已重做品牌侧栏、窗口外壳、卡片选择、段控件、列表列宽、状态图例及预览。该阶段属于落实参考的迭代，不能标为用户已接受的最终设计。

## 原图与提示词

| 原图 | 实际尺寸 | 用途与状态 | 完整提示词 |
| --- | --- | --- | --- |
| references/concept-v1.png | 1536×1024 | 整体参考，不是运行截图 | [concept-v1](prompts/concept-v1.md) |
| atlases/functional-v1.png | 1254×1254，4×4 | 功能对象；工具箱/报告两格被补图替代 | [functional-icons-v1](prompts/functional-icons-v1.md) |
| atlases/controls-v1.png | 1254×1254，4×4 | 原控制图，透明区域污染，停用 | [control-icons-v1](prompts/control-icons-v1.md) |
| atlases/controls-v2-rejected.png | 1254×1254，4×4 | 修图仍有杂点，归档，不参与构建 | [control-icons-v2](prompts/control-icons-v2.md) |
| atlases/controls-v3.png | 1254×1254，4×4 | 新轮廓；原图仍有杂点，构建提取主体后使用 | [control-icons-v3](prompts/control-icons-v3.md) |
| atlases/functional-repairs-v1.png | 1774×887，2×1 | 工具箱/报告补图，两格边缘通过 | [functional-repairs-v1](prompts/functional-repairs-v1.md) |
| atlases/spiral-islands-v1.png | 1536×1024，3×2 | 分类组件；已接入螺旋及选中态 | [spiral-islands-v1](prompts/spiral-islands-v1.md) |
| atlases/workbench-details-v1.png | 1536×1024，2×2 | 玻璃星球品牌与三张材质缩略图；数据仍为真实采样 | [workbench-details-v1](prompts/workbench-details-v1.md) |
| workspace-background-v1.png | 1536×1024 | 第一轮背景，已被新材质替代，保留 | [workspace-background-v1](prompts/workspace-background-v1.md) |
| workbench-material-v2.png | 1536×1024 | 当前设置工作台静态材质 | [workbench-material-v2](prompts/workbench-material-v2.md) |
| atlases/office-v1.png | 1774×887，4×2 | 8 个办公图标已切图，相关功能待实现 | [office-icons-v1](prompts/office-icons-v1.md) |
| atlases/cleanup-profiles-v1.png | 2172×724，3×1 | 轻度羽毛、中度缓存整理、重度芯片清扫；三格已接入清理档位 | [cleanup-profiles-v1](prompts/cleanup-profiles-v1.md) |

原图均保留，不覆盖。新增时递增版本并保存当次完整提示词与用途。原 assets/branding 保留；当前品牌/托盘使用玻璃星球，宠物继续使用 assets/pets 图集。

## 切图清单与确定性规则

[精确清单](sprites.json) 为构建入口，[Rust 切图工具](../../scripts/visual_assets.rs) 由 build.rs 调用。当前处理 55 格，包含两个停用的 tools-draft/report-draft；有效对象 53 个，背景与概念图另计。未接入的办公素材只生成开发产物，不链接进应用。

- 按行从左到右。功能：监测、猫、芯片、滑杆；清扫、历史、计时、便签；资源、办公、工具草稿、截图；剪贴板、启动器、报告草稿、会议。
- 控制：设置、固定、取消固定、详情；精简、完整、上、下；关闭、确认、加、减；最小化、最大化、播放、暂停。
- 分类组件：第一行资源/办公/工具，第二行对应选中态。办公：录音/回溯/待办/提醒；批注/贴图/便签/工作区。补图：工具、报告。
- 清理档位：轻度、中度、重度，每格 724×724，输出 64×64。

宽 W、高 H、列 C、行 R，单元使用整数比例边界 W*c/C、W*(c+1)/C、H*r/R、H*(r+1)/R，保留最右/最下像素。按主体 alpha≥128 的边界加 5% 留白、等比居中缩放。控制图先以 alpha≥200 连通主体切图，保留面积至少为最大主体 1/8 的断开笔画及 3 像素抗锯齿边缘，剔除外围杂点；此规则只用于指定的控制 atlas，不处理宠物或其他素材。切片需实看以防丢笔画。

缩放采用预乘 alpha 后 Lanczos3 再还原，避免黑边。功能/控制/办公输出 64×64，螺旋 128×128，背景 768×512；品牌另输出 32/64。每格验证非空、透明留白和边缘；当前运行素材未通过即阻止构建。原工具/报告草稿保留为 needs-review，不能加入 runtime。

构建目录 OUT_DIR 生成 artwork-manifest.json（实际边界、主体范围、尺寸、剔除像素、质量、是否运行）、artwork-preview.png（32/20像素接触表）、art-*.png/rgba。这些生成产物不提交；原图、清单及工具提交。测试覆盖断开的暂停笔画、抗锯齿、孤立杂点、白图标缩放及真实 GDI alpha 混合。

## 运行预算与用途

当前运行清单为 26 个 64×64 图标、10 个 128×128 切片与 1 张背景，共 37 个对象，原始 RGBA 总量 **2,654,208 字节（约 2.53 MiB）**，构建上限 3 MiB。螺旋六格新增 393,216 字节，品牌和材质四格新增 262,144 字节。这是素材量，不能当成应用内存占用。材质缩略图缓存 alpha 主体边界，并按实际内容比例绘制，避免横图连同方形透明留白被压扁。应用按需缓存纹理，设置关闭后释放背景纹理。PNG 解码仅在构建/测试中执行，不引入运行时图片解码器、网络下载、Node或Python。

导航使用监测/外观/设备/常规/清理/历史/计时；悬浮栏使用设置、固定、详情、精简/还原、清理、宠物；数值使用加减，窗口使用关闭/最大化/还原/最小化，计时主操作使用播放/暂停。原生菜单消费同一中立 RGBA 目录，通过 GDI 预乘 BGRA AlphaBlend 绘制不同图标，弹出结束释放位图/DC缓存。普通、悬停、选中、禁用与小尺寸须以实际页面核验，不能以素材大图或测试绿宣称视觉验收。
