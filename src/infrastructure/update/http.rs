use crate::domain::update::{Manifest, Release};
use reqwest::{
    Url,
    blocking::{Client, Response},
    redirect::Policy,
};
use serde_json::Value;
use std::{io::Read, time::Duration};

pub const GITHUB: &str = "GitHub";
pub const GITEE: &str = "Gitee";
const REPOSITORY: &str = "/xiaoyanse/desktop-pet/releases/";

pub(super) fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("DesktopMeter/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirect(redirect_policy())
        .build()
        .map_err(|_| "无法初始化 HTTPS 更新连接".into())
}
pub(super) fn redirect_policy() -> Policy {
    Policy::custom(|attempt| {
        if attempt.previous().len() >= 5 || !trusted_redirect(attempt.url()) {
            attempt.stop()
        } else {
            attempt.follow()
        }
    })
}
fn trusted_redirect(url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none_or(|port| port == 443)
        && (matches!(host, "github.com" | "api.github.com" | "gitee.com")
            || host.ends_with(".githubusercontent.com")
            || host.ends_with(".gitee.com")
            || host.ends_with(".giteeusercontent.com"))
}
pub(super) fn asset_url(value: &str, source: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "更新附件地址无效")?;
    let expected = if source == GITHUB {
        "github.com"
    } else if source == GITEE {
        "gitee.com"
    } else {
        return Err("更新来源无效".into());
    };
    if !trusted_redirect(&url)
        || url.host_str() != Some(expected)
        || !(url.path().starts_with(REPOSITORY)
            || (source == GITEE
                && url
                    .path()
                    .starts_with("/xiaoyanse/desktop-pet/attach_files/")))
        || url.path().contains("%")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("更新附件不属于指定仓库的 HTTPS Release".into());
    }
    Ok(url)
}
pub(super) fn bounded(mut response: Response, limit: u64) -> Result<Vec<u8>, String> {
    if !response.status().is_success() {
        return Err(format!("更新服务返回 HTTP {}", response.status().as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err("更新响应超过大小限制".into());
    }
    let mut data = Vec::new();
    response
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|_| "更新响应读取失败")?;
    if data.len() as u64 > limit {
        return Err("更新响应超过大小限制".into());
    }
    Ok(data)
}
pub(super) fn check(source: &str) -> Result<Option<Release>, String> {
    let endpoint = if source == GITHUB {
        "https://api.github.com/repos/xiaoyanse/desktop-pet/releases/latest"
    } else {
        "https://gitee.com/api/v5/repos/xiaoyanse/desktop-pet/releases/latest"
    };
    let client = client()?;
    let response = client
        .get(endpoint)
        .timeout(Duration::from_secs(20))
        .send()
        .map_err(|_| format!("{source} 更新连接失败"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let release: Value =
        serde_json::from_slice(&bounded(response, 512 * 1024)?).map_err(|_| "发布信息格式无效")?;
    if release["draft"].as_bool().unwrap_or(false)
        || release["prerelease"].as_bool().unwrap_or(false)
    {
        return Ok(None);
    }
    let tag = release["tag_name"].as_str().ok_or("发布版本缺少标签")?;
    let assets = release["assets"]
        .as_array()
        .or_else(|| release["assets"]["links"].as_array())
        .ok_or("发布版本缺少附件")?;
    let manifest_asset = assets
        .iter()
        .find(|asset| asset["name"] == "update-manifest.json")
        .ok_or("发布版本缺少更新清单")?;
    let manifest_url = manifest_asset["browser_download_url"]
        .as_str()
        .ok_or("更新清单地址缺失")?;
    let manifest_data = bounded(
        client
            .get(asset_url(manifest_url, source)?)
            .send()
            .map_err(|_| "更新清单下载失败")?,
        16 * 1024,
    )?;
    let manifest: Manifest =
        serde_json::from_slice(&manifest_data).map_err(|_| "更新清单格式无效")?;
    manifest.validate(tag)?;
    let package = assets
        .iter()
        .find(|asset| asset["name"].as_str() == Some(&manifest.file_name))
        .ok_or("发布版本缺少 Windows 更新包")?;
    if package["size"]
        .as_u64()
        .is_some_and(|size| size != manifest.size)
        || package["digest"]
            .as_str()
            .is_some_and(|digest| digest != format!("sha256:{}", manifest.sha256))
    {
        return Err("发布附件与更新清单不一致".into());
    }
    let url = package["browser_download_url"]
        .as_str()
        .ok_or("更新包地址缺失")?
        .to_string();
    asset_url(&url, source)?;
    Ok(Some(Release {
        version: manifest.version.clone(),
        notes: release["body"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(12000)
            .collect(),
        page: format!(
            "https://{}{REPOSITORY}tag/{tag}",
            if source == GITHUB {
                "github.com"
            } else {
                "gitee.com"
            }
        ),
        url,
        source: source.into(),
        manifest,
    }))
}
