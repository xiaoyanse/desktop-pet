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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delta {
    pub base_version: String,
    pub base_executable_sha256: String,
    pub executable_size: u64,
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
    #[serde(skip)]
    pub url: String,
}
impl Delta {
    pub fn validate(&self, version: &str) -> bool {
        newer(version, &self.base_version)
            && self.file_name
                == format!(
                    "DesktopMeter-{}-from-{}-delta-win-x64.zip",
                    version, self.base_version
                )
            && (1..=100 * 1024 * 1024).contains(&self.size)
            && (1..=100 * 1024 * 1024).contains(&self.executable_size)
            && [&self.sha256, &self.base_executable_sha256]
                .iter()
                .all(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
    }
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
            delta: None,
        };
        assert!(m.validate("v0.2.0").is_ok());
        assert!(m.validate("v0.3.0").is_err());
        m.file_name = "../app.zip".into();
        assert!(m.validate("v0.2.0").is_err());
        m.file_name = "DesktopMeter-0.2.0-win-x64.zip".into();
        m.size = u64::MAX;
        assert!(m.validate("v0.2.0").is_err());
    }

    #[test]
    fn legacy_full_manifest_remains_compatible_and_delta_requires_monotonic_exact_baseline() {
        let m: Manifest = serde_json::from_value(serde_json::json!({"schema":1,"version":"0.2.0","file_name":"DesktopMeter-0.2.0-win-x64.zip","size":100,"sha256":"a".repeat(64),"executable_sha256":"b".repeat(64)})).unwrap();
        assert!(m.delta.is_none());
        assert!(m.validate("v0.2.0").is_ok());
        let mut delta = Delta {
            base_version: "0.2.0".into(),
            base_executable_sha256: "b".repeat(64),
            executable_size: 100,
            file_name: "DesktopMeter-0.2.1-from-0.2.0-delta-win-x64.zip".into(),
            size: 80,
            sha256: "a".repeat(64),
            url: String::new(),
        };
        assert!(delta.validate("0.2.1"));
        assert!(!delta.validate("0.2.0"));
        delta.base_executable_sha256 = "bad".into();
        assert!(!delta.validate("0.2.1"));
    }
}
