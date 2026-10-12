use super::{Process, Report};
use std::fmt::Write;

fn cell(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(1024)
        .collect::<String>()
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn bytes(value: Option<u64>) -> String {
    value.map_or("未知".into(), |v| {
        format!("{:.1} MiB", v as f64 / 1048576.0)
    })
}
fn processes(text: &mut String, title: &str, rows: &[Process]) {
    let _ = writeln!(text, "\n## {title}\n");
    if rows.is_empty() {
        text.push_str("没有可列出的进程。\n");
        return;
    }
    text.push_str("| 进程 | PID | CPU（全机） | 工作集 | 私有提交 | 可核实父进程 | 可执行路径（脱敏） |\n| --- | --- | --- | --- | --- | --- | --- |\n");
    for p in rows {
        let cpu = p
            .cpu
            .filter(|v| v.is_finite())
            .map_or("未知".into(), |v| format!("{v:.2}%"));
        let parent = p.parent.as_ref().map_or(
            "未核实（权限/退出/时间不符）".into(),
            |(pid, name)| format!("{} · PID {pid}", cell(name)),
        );
        let _ = writeln!(
            text,
            "| {} | {} | {} | {} | {} | {} | {} |",
            cell(&p.name),
            p.pid,
            cpu,
            bytes(p.working_set),
            bytes(p.private),
            parent,
            cell(p.path.as_str())
        );
    }
}
pub fn markdown(report: &Report) -> String {
    let mut text = format!(
        "# 桌面仪表 · 资源诊断报告\n\n采集时间：{}（本机时间）\n应用版本：{}\n\n",
        cell(&report.captured_at),
        env!("CARGO_PKG_VERSION")
    );
    text.push_str("本报告由手动采集生成。路径默认脱敏；不读取启动参数、环境变量、窗口内容或命令历史。\n\n## 资源快照\n\n| 监测项 | 数值 |\n| --- | --- |\n");
    for (metric, reading) in &report.resources {
        let _ = writeln!(text, "| {} | {} |", metric.label(), cell(reading));
    }
    let _ = writeln!(
        text,
        "\n枚举 {} 个进程；{} 个进程的 CPU 或内存不可读取。Node {} 个，Python {} 个（当前报告列出数量）；另有 {} 个运行时进程超出 512 项上限。",
        report.enumerated,
        report.unreadable,
        report.runtime_count("Node"),
        report.runtime_count("Python"),
        report.omitted_runtimes
    );
    processes(&mut text, "内存前五名（物理工作集）", &report.memory_top);
    processes(&mut text, "CPU 前五名（采集期间全机占比）", &report.cpu_top);
    processes(&mut text, "Node / Python 进程来源", &report.runtimes);
    text.push_str("\n## 如何理解这些数据\n\n- CPU 来自约 2 秒的差分采样；新启动或 PID 变化的进程显示未知。\n- 工作集含共享页，不能相加当作整机占用。私有提交不是实际驻留的物理内存，也不能与内核池直接相加。\n- 路径仅代表可执行文件；父进程经过创建时间检查。相同 Node/Python 运行时可能服务不同任务，报告不能仅凭名称或路径认定为 MCP，也不读取脚本参数。\n- 内核池属于驱动/内核分配。单次快照不能定位泄漏；需继续观察池标签及增长，清缓存无法证明修复驱动泄漏。\n- 权限不足与采集期间退出均保留未知，不自动提权、不结束进程、不执行内存清理。\n");
    text
}
