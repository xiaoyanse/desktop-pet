# Windows 安装与增量更新

## 0.2.2 已发布与增量测试（2026-10-13）

本次四项体验修复的源码提交为268eefa2961f5c23c82ed2a28217058cebb4be62；217项检查、严格clippy、格式与release通过，20页原生合成预览完成。设置窗口、EXE与安装器均已补齐品牌图标；两个产物的六张图标资源逐张SHA256核验一致，证据evidence/settings-stage28-native01/packaged-icons.json。程序ProductVersion=0.2.2。

| 产物 | 字节 | SHA256 |
| --- | ---: | --- |
| 安装 EXE | 5876678 | ba27e1e38e6e47386b57f3ff9b426ca0f1cfdb127bd4eb2649cf1e388e395aac |
| 全量 ZIP | 5318964 | 8bc2c79ef3ef4dbfb5bbb0ef9bc1c4587263c499ab14de57741cfd7b9ee4feff |
| 0.2.1→0.2.2 增量 ZIP | 3233823 | 48ced538c84429a63af1fbd5e33876e64eb5c76b8fae8dd366c5d8cb1fde3d9c |
| 新程序 | 12342784 | 13a460c4c51dd1ebc62db4249733f3851f887a714e13fc3a3417dc66aeda0c81 |

精确基线EXE SHA256为4ed6789587e19fc095a22b33efd6bb551c12205de3c3bb3511eec28a6ba66642，即原始0.2.1发布包。打包时生产解码器已真实重建新EXE并核对散列。增量比全量少39.2%下载字节。

[GitHub v0.2.2](https://github.com/xiaoyanse/desktop-pet/releases/tag/v0.2.2)和[Gitee v0.2.2](https://gitee.com/xiaoyanse/desktop-pet/releases/tag/v0.2.2)均已公开发布安装EXE、全量ZIP、增量ZIP与update-manifest.json；GitHub非草稿，四附件尺寸回读一致。Gitee Release ID为1202972，另有平台自动源码包，四附件名称回读齐全。公开元数据证据：E:/Temp/desktop-meter-update-public-0.2.2-20261013/published-assets.json。

原始0.2.1程序分别通过两个来源实际检查并下载0.2.2，transfer.json均为kind=delta；重建程序散列均等于本次发布EXE。0.2.2开发探针模拟错误的增量预期散列，实际产生“增量包SHA256校验失败”，自动下载全量，kind=full且重建散列一致。三项结果全部passed=true，未安装这些任务；证据E:/Temp/desktop-meter-update-public-0.2.2-20261013/summary.json。

最终生产helper的隔离替换与失败回退通过：E:/Temp/desktop-meter-update-native-20261013-200847/report.json。此次安装器重新构建及品牌资源核验通过，但实际安装/卸载回归因已有同名开始菜单目录触发保护而未执行；未覆盖或删除现有快捷方式，不借用0.2.1安装测试称0.2.2已安装验收。0.2.1安装/卸载历史验证仍保留在下方。

当前运行实例保留，用户在0.2.1内自行检查更新、下载并安装测试。完整GUI升级、实际跨屏和目标软件粘贴仍属现场验收；不恢复自主任务或其他功能扩展。

2026-10-13 用户验收更正：Release 必须同时提供 Windows 安装包、增量更新包和全量更新包。
此前 0.2.0 只有全量 ZIP，不能称安装包全部完成。本轮补齐 0.2.1，不覆盖已公开版本。

## 产物契约

| 产物 | 文件 | 用途 |
| --- | --- | --- |
| 中文 Windows 安装向导 | DesktopMeter-0.2.1-setup-win-x64.exe | 首次安装、目录选择、可选桌面快捷方式、开始菜单与卸载 |
| 全量更新包 | DesktopMeter-0.2.1-win-x64.zip | 任意支持的旧版本更新；保留此文件名使 0.2.0 更新器兼容 |
| 增量更新包 | DesktopMeter-0.2.1-from-0.2.0-delta-win-x64.zip | 精确基线 0.2.0 的二进制差分及新版配套文件 |
| 更新清单 | update-manifest.json | 三类文件大小/SHA256、版本、增量基线/程序散列与重建大小 |

安装器使用本机 Inno Setup 6 作为打包工具，生产应用、差分生成/应用、在线更新均为 Rust。
参考 Inno Setup 的 DefaultDirName / UninstallFilesDir 官方文档：
https://jrsoftware.org/ishelp/topic_setup_defaultdirname.htm
https://jrsoftware.org/ishelp/topic_setup_uninstallfilesdir.htm
单用户安装无需管理员，默认优先 E:\Apps\DesktopMeter，再 D 盘，均不存在才使用用户程序目录。
程序四个发布文件在安装目录 current 子目录，卸载器在同级 uninstall 子目录；
更新只替换 current，避免卸载器或快捷方式干扰受控目录检查。快捷方式指向稳定路径。
不自动结束其他程序、不默认开机启动，安装完成启动选项默认不勾选。
卸载只移除安装记录中的文件与快捷方式，不删除应用独立数据目录或用户后来加入的文件。

## 差分与回退契约

DMDELTA1 为项目自身 Rust 区块差分格式：匹配旧程序的 4 KiB 块，记录复制或新增字节；
不是把完整程序换个文件名称为增量。新版 README、启动脚本及内部清单随差分 ZIP 更新。
生成后用生产 Rust 解码器重建并核对新程序 SHA256；不一致拒绝打包。
应用差分时使用固定 64 KiB 缓冲区，不把旧/新程序同时整文件加载到运行内存；
限制目标大小、指令数、复制范围、字面数据长度和尾部指令，拒绝路径穿越、重复文件和链接。
仅开发时差分生成按需读取两份程序，不在常驻监控或启动时执行。

更新检查保持 schema 1 的全量字段，另附可选 delta；旧客户端会忽略新增字段。
新客户端在基线版本和本地 EXE SHA256 同时匹配、增量包小于全量包时优先增量；
增量下载、散列或重建失败则清除自身暂存产物并改用全量包；用户取消时立即结束，不触发回退下载。
最终重建程序散列与内部版本验证一致后，沿用既有保存、备份、健康确认和失败回滚流程。
增量错误原因记在自身 stage/transfer.json 中；下载进度使用当前包的百分比，回退时重置。
更新页说明全量/增量大小、基线及自动回退，不要求用户手动选择错误的差分包。

兼容边界：已发布 0.2.0 更新器本身没有差分解码器，首次到 0.2.1 使用全量；
从 0.2.1 起的后续更新才可自动使用与当前版本匹配的增量。增量 ZIP 不能直接解压覆盖安装。
多版本不匹配时使用全量。用户使用正常软件前，本轮不替换其当前实例。

## 验证状态

0.2.1 本地阶段已验证，包内源码 2004c29a19779d99b0959c1e646067aefc991c15。
209 单元＋2 切图＋1 架构共 212 项通过，格式/严格 clippy/release 构建通过。
20 页最终原生合成预览通过，evidence/updates-stage27-native01/status.json complete=true。
隔离真实替换/健康确认/失败恢复通过，E:\Temp\desktop-meter-update-native-20261013-191656/report.json。

安装器真实静默安装到 E 盘隔离目录，四个安装文件 SHA256 与发布包一致，HKCU 卸载注册
版本/路径正确，安装版公开更新检查通过；卸载成功，程序/注册/快捷方式移除，额外用户文件保留。
证据 E:\Temp\desktop-meter-installer-20261013-191837/report.json；没有启动普通宠物程序或结束用户现有实例。
打包生产解码器实际重建新程序并核对 SHA256。另以最终发布程序拒绝 UINT64_MAX 越界复制指令，
E:\Temp\desktop-meter-delta-security-20261013/report.json。没有对其他程序使用这些测试命令。

| 产物 | 字节 | SHA256 |
| --- | ---: | --- |
| 安装 EXE | 5673154 | 04547c8b66e14e8488a4aa68ffdc45a11c9815e381bf07ad6655697f11226866 |
| 全量 ZIP | 5233091 | 218101505ffbd080e7c00d3c051faf7c24fd0dad1b9f6a5931477f3186bf7096 |
| 增量 ZIP | 3130796 | d0fa2db6a6d9315f49cd2b69c0fc46536c6ccf9235bd7bd959626ab726b22536 |
| 新程序 | 12257792 | 4ed6789587e19fc095a22b33efd6bb551c12205de3c3bb3511eec28a6ba66642 |

增量比全量少约 40.2% 下载字节。安装器尚未签名。本地通过不替代完整 GUI
现场更新验收。其他功能及后台自主任务仍暂停。

## 公开发布与真实更新链路

2026-10-13 已发布 [GitHub v0.2.1](https://github.com/xiaoyanse/desktop-pet/releases/tag/v0.2.1)
与 [Gitee v0.2.1](https://gitee.com/xiaoyanse/desktop-pet/releases/tag/v0.2.1)。
两边均包含安装 EXE、全量 ZIP、增量 ZIP 与 update-manifest.json；Gitee 另自动附带源码包。
保留原 0.2.0，不覆盖已公开产物。main 源码及验证脚本已推送两仓库。

E:\Temp\desktop-meter-update-public-0.2.1-20261013 保存最终生产程序的公开链路证据：

- GitHub / Gitee 分别实际下载增量，transfer.json kind=delta，重建程序散列等于上表。
- GitHub 故意替换探针内预期增量散列，实际增量校验失败，自动下载全量并重建；
  kind=full，保留 deltaFallbackReason，正常结束。
- 原已发布 0.2.0 程序分别通过两个来源下载 0.2.1 全量包，通过原解压/版本/程序校验；
  证实新清单没有破坏旧客户端。旧版不会识别 delta，所以此处确实使用全量。
- 两边安装 EXE 无令牌公开下载，字节数/散列与本地产物一致。
- 所有五个下载探针的 payload/desktop-meter.exe SHA256 相同且正确；summary.json 全部 passed=true。

没有安装这些下载任务，没有替换用户当前桌面实例；完整 GUI 自更新和普通安装向导点击
属于现场验收范围。实际静默安装/卸载、生产差分解码、远端真实下载与隔离健康回退均已单独验证。
