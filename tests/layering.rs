use std::{
    fs,
    path::{Path, PathBuf},
};

fn sources(folder: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            result.push(path);
        }
    }
    result
}

#[test]
fn layer_boundaries_keep_platform_and_storage_out_of_ui_and_business_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for (layer, forbidden) in [
        (
            "ui",
            vec![
                "crate::infrastructure",
                "windows_sys::",
                "windows::Win32",
                "std::fs",
                "std::process::Command",
            ],
        ),
        (
            "application",
            vec![
                "infrastructure",
                "crate::ui",
                "eframe::",
                "windows_sys::",
                "windows::Win32",
                "std::fs",
            ],
        ),
        (
            "domain",
            vec![
                "infrastructure",
                "application",
                "interfaces",
                "eframe::",
                "windows_sys::",
                "windows::Win32",
                "std::fs",
            ],
        ),
        (
            "interfaces",
            vec![
                "infrastructure",
                "application",
                "crate::ui",
                "eframe::",
                "windows_sys::",
                "windows::Win32",
            ],
        ),
        (
            "infrastructure",
            vec!["crate::ui", "crate::application", "eframe::", "egui::"],
        ),
    ] {
        for path in sources(&root.join(layer)) {
            let content = fs::read_to_string(&path).unwrap();
            for token in &forbidden {
                assert!(
                    !content.contains(token),
                    "{} crosses boundary using {token}",
                    path.display()
                );
            }
        }
    }
}
