use serde::{Deserialize, Serialize};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub automatic: bool,
    pub pinned: bool,
    pub skipped: String,
    pub prefer_gitee: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            automatic: true,
            pinned: false,
            skipped: String::new(),
            prefer_gitee: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub page: String,
    pub source: String,
    pub manifest: Manifest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub version: String,
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
    pub executable_sha256: String,
}
impl Manifest {
    pub fn validate(&self, tag: &str) -> Result<(), String> {
        let version =
            semver::Version::parse(tag.trim_start_matches('v')).map_err(|_| "更新版本号无效")?;
        if self.schema != 1
            || self.version != version.to_string()
            || !version.pre.is_empty()
            || self.file_name != format!("DesktopMeter-{}-win-x64.zip", self.version)
            || !(1..=100 * 1024 * 1024).contains(&self.size)
            || [&self.sha256, &self.executable_sha256]
                .iter()
                .any(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("更新清单与发布版本不匹配".into());
        }
        Ok(())
    }
}
pub fn newer(candidate: &str, current: &str) -> bool {
    match (
        semver::Version::parse(candidate),
        semver::Version::parse(current),
    ) {
        (Ok(a), Ok(b)) => a > b && a.pre.is_empty(),
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_are_monotonic_and_prereleases_are_not_automatic() {
        assert!(newer("0.10.0", "0.2.0"));
        for bad in ["0.2.0", "0.1.0", "0.3.0-beta.1", "garbage"] {
            assert!(!newer(bad, "0.2.0"));
        }
    }
    #[test]
    fn manifest_rejects_path_traversal_wrong_version_and_unbounded_sizes() {
        let mut m = Manifest {
            schema: 1,
            version: "0.2.0".into(),
            file_name: "DesktopMeter-0.2.0-win-x64.zip".into(),
            size: 100,
            sha256: "a".repeat(64),
            executable_sha256: "b".repeat(64),
        };
        assert!(m.validate("v0.2.0").is_ok());
        assert!(m.validate("v0.3.0").is_err());
        m.file_name = "../app.zip".into();
        assert!(m.validate("v0.2.0").is_err());
        m.file_name = "DesktopMeter-0.2.0-win-x64.zip".into();
        m.size = u64::MAX;
        assert!(m.validate("v0.2.0").is_err());
    }
}
