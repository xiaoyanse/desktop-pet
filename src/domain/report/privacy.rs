#[derive(Clone)]
pub struct SafePath(String);

impl SafePath {
    pub fn unavailable() -> Self {
        Self("不可读取（权限限制、退出或 PID 已变化）".into())
    }
    pub fn from_raw(path: &str) -> Self {
        Self(redact(path))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn redact(path: &str) -> String {
    let path = path.replace('/', "\\");
    let path = path.strip_prefix("\\\\?\\").unwrap_or(&path);
    let path = path.strip_prefix("\\??\\").unwrap_or(path);
    let parts: Vec<_> = path.split('\\').filter(|p| !p.is_empty()).collect();
    let Some(file) = parts.last() else {
        return "不可读取（空路径）".into();
    };
    if path.starts_with("\\\\") || path.to_ascii_lowercase().starts_with("unc\\") {
        return format!("<网络位置>\\<目录>\\{file}");
    }
    if parts.len() < 2 || !parts[0].ends_with(':') {
        return format!("<位置>\\{file}");
    }
    let base = parts[1].to_ascii_lowercase();
    if base == "windows" || base == "program files" || base == "program files (x86)" {
        return parts.join("\\");
    }
    if base == "users" && parts.len() >= 4 {
        let tail = &parts[3..];
        if matches!(
            tail[0].to_ascii_lowercase().as_str(),
            "appdata" | ".nvm" | ".volta" | ".claude" | ".codex"
        ) {
            return format!("{}\\Users\\<用户>\\{}", parts[0], tail.join("\\"));
        }
        return format!("{}\\Users\\<用户>\\<目录>\\{file}", parts[0]);
    }
    format!("{}\\<目录>\\{file}", parts[0])
}
