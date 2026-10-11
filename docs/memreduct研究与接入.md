# Mem Reduct 核心能力接入

研究时间：2026-10-12。来源为 [Mem Reduct 仓库](https://github.com/henrypp/memreduct) 中的 src/main.c、src/main.h 和 [System Informer 原生接口定义](https://github.com/winsiderss/systeminformer/blob/master/phnt/include/ntexapi.h)。本项目使用独立 Rust 实现 Windows 能力，没有嵌入或逐段翻译 GPL-3.0 C 源码。原项目代码许可仍须在未来直接复用其代码时单独处理。

| 核心操作 | 本项目实现 | 结果与注意点 |
| --- | --- | --- |
| 低优先级待机页 | SystemMemoryListInformation / command 5 | 默认唯一选择；待机缓存已属于可用内存 |
| 应用工作集 | 同一信息类 / command 2 | 应用以后可能重新调页 |
| 系统文件缓存工作集 | SetSystemFileCacheSize(MAX, MAX, 0) | 使用官方公开 API；需要 SeIncreaseQuotaPrivilege |
| 卷写入缓存 | 枚举本地固定卷，FlushFileBuffers | 将待写数据落盘，不删除文件；逐卷失败会报告 |
| 修改页列表 | SystemMemoryListInformation / command 3 | 可能增加磁盘活动，默认不参与自动执行 |
| 全部待机页 | 同一信息类 / command 4 | 丢弃缓存，默认不参与自动执行 |
| 注册表缓存 | SystemRegistryReconciliationInformation / class 156 | 管理员权限，系统支持程度以返回状态为准 |
| 合并相同内存页 | SystemCombinePhysicalMemoryInformation / class 130 | ABI 为事件句柄、页数、标志；效果取决于实际内容 |

公开 API 依据：[SetSystemFileCacheSize](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-setsystemfilecachesize)、[FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)。其他原生 NT 接口并非完整公开稳定契约，不支持或权限不足时展示 NTSTATUS，不伪造成功。

除八种操作之外，接入物理占用阈值、定时间隔、冷却、防重复执行、托盘入口、可选 Ctrl+Alt+C 快速面板入口、逐项结果与最近 50 次持久化记录。当前物理内存、提交量、系统缓存、待机页和修改页状态可在清理中心查看。悬浮窗清扫图标和宠物悬停提供独立快捷面板，其中清扫按钮明确只执行低优先级待机缓存；其选项临时构造，不修改清理中心已选操作和自动规则。完整清理中心维持确认步骤。

默认自动规则关闭，自动执行只在应用已有管理员权限时生效。普通权限手动清理启动本机同一 exe 的提升辅助进程；结果通过限长、本机专用的消息型命名管道返回，双方校验对端 PID，辅助进程校验父进程可执行文件路径并限制模拟权限。不会要求 helper 写入任意文件路径，不绕过 UAC、不安装服务。

每次执行记录各操作结果，保留部分成功。若操作后状态读取失败，仍保留操作记录，显示“读数失败”，不给出虚构的释放数量；即使读数正常，可用内存差值也不能全部归因于清理，因为其他应用仍在工作。

此次范围是清理核心及其触发、配置和结果闭环。Mem Reduct 自身的更新器、语言包、跳过 UAC 任务和平台包装没有复制。清理缓存也不能修复分页池或非分页池中的驱动泄漏，监视器不会结束工作进程。

验证边界：已检查接口与结构布局、真实只读状态、域规则、服务并发与失败、命名管道收发；没有在用户当前工作环境执行八种系统清理操作。不能将“编译与测试通过”解释为全部清理 API 的现场效果已验收。
