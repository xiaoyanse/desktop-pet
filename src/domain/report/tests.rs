use super::*;

#[test]
fn privacy_masks_user_projects_custom_drives_and_unc_hostnames() {
    for (raw, hidden) in [
        (
            "C:\\Users\\秘密用户\\AppData\\Local\\Programs\\Python\\python.exe",
            "秘密用户",
        ),
        (
            "C:\\Users\\秘密用户\\Documents\\秘密项目\\.venv\\Scripts\\python.exe",
            "秘密项目",
        ),
        ("E:\\秘密项目\\node.exe", "秘密项目"),
        ("\\\\内部主机\\秘密共享\\bin\\node.exe", "内部主机"),
        ("\\\\?\\UNC\\内部主机\\共享\\node.exe", "内部主机"),
    ] {
        let path = privacy::SafePath::from_raw(raw);
        assert!(!path.as_str().contains(hidden));
        assert!(path.as_str().ends_with(".exe"));
    }
    assert_eq!(
        privacy::SafePath::from_raw("C:/Windows/System32/svchost.exe").as_str(),
        "C:\\Windows\\System32\\svchost.exe"
    );
    assert!(
        privacy::SafePath::from_raw(
            "C:\\Users\\张三\\AppData\\Local\\Programs\\Python\\python.exe"
        )
        .as_str()
        .contains("Python\\python.exe")
    );
}
#[test]
fn runtime_names_are_case_insensitive_and_do_not_claim_unrelated_services() {
    for name in ["NODE.EXE", "nodejs.exe"] {
        assert_eq!(runtime_kind(name), Some("Node"));
    }
    for name in ["pythonw.exe", "Python3.13.exe", "python.exe"] {
        assert_eq!(runtime_kind(name), Some("Python"));
    }
    assert_eq!(runtime_kind("pythonservice.exe"), None);
    assert_eq!(runtime_kind("node-helper.exe"), None);
}
#[test]
fn markdown_preserves_unknown_and_sanitizes_cells_without_exporting_private_paths() {
    let mut report = Report {
        captured_at: "2026-10-12 12:00:00".into(),
        ..Default::default()
    };
    report.set_resources(&Snapshot::default());
    report.runtimes.push(Process {
        name: "node|坏\n名称.exe".into(),
        pid: 3,
        parent: None,
        path: privacy::SafePath::from_raw("E:\\项目密钥\\node.exe"),
        cpu: None,
        working_set: None,
        private: None,
    });
    let text = report.markdown();
    assert!(text.contains("分页池"));
    assert!(text.contains("非分页池"));
    assert!(!text.contains("项目密钥"));
    assert!(!text.contains("坏\n名称"));
    assert!(text.contains("&#124;"));
    assert!(text.contains("未知"));
    assert!(text.contains("不能仅凭名称或路径认定为 MCP"));
}
