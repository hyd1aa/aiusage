//! Official-release checks. Binary updates never fall back to installing Python.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use url::Url;

pub const REPOSITORY_URL: &str = "https://github.com/hyd1aa/aiusage";
pub const LATEST_API: &str = "https://api.github.com/repos/hyd1aa/aiusage/releases/latest";
pub const CACHE_SECONDS: f64 = 6.0 * 60.0 * 60.0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub digest: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub title: String,
    pub notes: String,
    pub tarball_url: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}
pub fn version_tuple(value: &str) -> Vec<i64> {
    value
        .split('.')
        .map(crate::cli::python_integer)
        .collect::<Option<_>>()
        .unwrap_or_default()
}
pub fn is_newer(latest: &str, current: &str) -> bool {
    version_tuple(latest) > version_tuple(current)
}
pub fn stable_version(value: &str) -> bool {
    let parts: Vec<_> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}
pub fn parse(payload: &Value) -> Result<ReleaseInfo, &'static str> {
    if !payload.is_object()
        || payload["draft"].as_bool() == Some(true)
        || payload["prerelease"].as_bool() == Some(true)
    {
        return Err("invalid stable release");
    }
    let tag = payload["tag_name"]
        .as_str()
        .ok_or("malformed release response")?;
    let version = tag
        .strip_prefix('v')
        .filter(|s| stable_version(s))
        .ok_or("malformed release response")?;
    let tarball = payload["tarball_url"]
        .as_str()
        .ok_or("malformed release response")?;
    let url = Url::parse(tarball).map_err(|_| "untrusted release source")?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || !matches!(
            url.host_str(),
            Some("api.github.com" | "github.com" | "codeload.github.com")
        )
    {
        return Err("untrusted release source");
    }
    let path = url.path().to_lowercase();
    if !(path.starts_with("/hyd1aa/aiusage/") || path.starts_with("/repos/hyd1aa/aiusage/")) {
        return Err("release source is not the official repository");
    }
    let assets = payload["assets"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|a| {
                    Some(Asset {
                        name: a["name"].as_str()?.into(),
                        browser_download_url: a["browser_download_url"].as_str()?.into(),
                        digest: a["digest"].as_str().map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(ReleaseInfo {
        version: version.into(),
        title: payload["name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(tag)
            .into(),
        notes: payload["body"].as_str().unwrap_or("").into(),
        tarball_url: tarball.into(),
        assets,
    })
}
pub fn cache_path() -> PathBuf {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::home_dir().join(".cache"))
        .join("aiusage/latest.json")
}
pub fn save_cache(info: &ReleaseInfo, path: &Path, now: f64) -> bool {
    use std::os::unix::fs::DirBuilderExt;
    let temp = path.with_extension("tmp");
    let result = (|| -> std::io::Result<()> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path.parent().unwrap_or(Path::new(".")))?;
        // Keep the cache readable by the untouched Python reference. Assets are
        // fetched again on explicit update and need not live in this cache.
        let value = json!({"version":info.version,"title":info.title,"notes":info.notes,"tarball_url":info.tarball_url,"checked_at":now});
        fs::write(&temp, serde_json::to_vec(&value)?)?;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))?;
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result.is_ok()
}
pub fn cached_at(path: &Path, now: f64, max_age: f64) -> Option<ReleaseInfo> {
    let mut value: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let checked = value.as_object_mut()?.remove("checked_at")?.as_f64()?;
    if now - checked > max_age {
        return None;
    }
    serde_json::from_value(value).ok()
}
pub fn cached_latest() -> Option<ReleaseInfo> {
    cached_at(&cache_path(), crate::cli::now(), CACHE_SECONDS)
}
fn fetch(url: &str, timeout: Duration, limit: u64) -> Result<Vec<u8>, &'static str> {
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let response = agent
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "AIUsage")
        .call()
        .map_err(|_| "release request failed")?;
    let mut bytes = vec![];
    response
        .into_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "release request failed")?;
    if bytes.len() as u64 > limit {
        return Err("release response too large");
    }
    Ok(bytes)
}
pub fn check_latest(timeout: Duration) -> Result<ReleaseInfo, &'static str> {
    let payload: Value = serde_json::from_slice(&fetch(LATEST_API, timeout, 2 * 1024 * 1024)?)
        .map_err(|_| "malformed release response")?;
    let info = parse(&payload)?;
    save_cache(&info, &cache_path(), crate::cli::now());
    Ok(info)
}
pub fn target_name() -> Result<&'static str, &'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-amd64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("macos", "aarch64") => Ok("macos-arm64"),
        _ => Err("unsupported release platform"),
    }
}
pub fn binary_asset<'a>(info: &'a ReleaseInfo, target: &str) -> Result<&'a Asset, &'static str> {
    let name = format!("aiusage-v{}-{target}", info.version);
    let matches: Vec<_> = info.assets.iter().filter(|a| a.name == name).collect();
    if matches.len() != 1 {
        return Err("this release has no compatible Rust binary");
    }
    let asset = matches[0];
    let expected = format!(
        "{REPOSITORY_URL}/releases/download/v{}/{name}",
        info.version
    );
    if asset.browser_download_url != expected {
        return Err("untrusted binary asset");
    }
    let digest = asset
        .digest
        .as_deref()
        .and_then(|s| s.strip_prefix("sha256:"))
        .ok_or("binary checksum unavailable")?;
    if digest.len() != 64 || !digest.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("binary checksum unavailable");
    }
    Ok(asset)
}
pub fn verify_digest(bytes: &[u8], asset: &Asset) -> bool {
    asset.digest.as_ref().is_some_and(|expected| {
        format!("sha256:{:x}", Sha256::digest(bytes)).eq_ignore_ascii_case(expected)
    })
}
pub fn install_release(
    info: &ReleaseInfo,
    current: &str,
    prefix: &Path,
) -> Result<String, &'static str> {
    install_release_with(info, current, prefix, |url| {
        fetch(url, Duration::from_secs(15), 64 * 1024 * 1024)
    })
}
pub fn install_release_with(
    info: &ReleaseInfo,
    current: &str,
    prefix: &Path,
    download: impl FnOnce(&str) -> Result<Vec<u8>, &'static str>,
) -> Result<String, &'static str> {
    if !is_newer(&info.version, current) {
        return Err("not newer");
    }
    let asset = binary_asset(info, target_name()?)?;
    let bytes = download(&asset.browser_download_url)?;
    if !verify_digest(&bytes, asset) {
        return Err("binary checksum mismatch");
    }
    let temp = tempfile::Builder::new()
        .prefix("aiusage-update-")
        .tempdir()
        .map_err(|_| "update staging failed")?;
    let binary = temp.path().join("aiusage");
    fs::write(&binary, bytes).map_err(|_| "update staging failed")?;
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700))
        .map_err(|_| "update staging failed")?;
    let version = Command::new(&binary)
        .arg("--version")
        .output()
        .map_err(|_| "release version verification failed")?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != format!("AIUsage {}", info.version)
    {
        return Err("release version verification failed");
    }
    let installer = temp.path().join("install.sh");
    let uninstaller = temp.path().join("uninstall.sh");
    fs::write(&installer, include_str!("../install.sh")).map_err(|_| "update staging failed")?;
    fs::write(&uninstaller, include_str!("../uninstall.sh"))
        .map_err(|_| "update staging failed")?;
    let mut command = if prefix == Path::new("/usr/local") && unsafe { libc::geteuid() } != 0 {
        let mut cmd = Command::new("sudo");
        cmd.arg("sh");
        cmd
    } else {
        Command::new("sh")
    };
    let status = command
        .arg(&installer)
        .env("PREFIX", prefix)
        .env("AIUSAGE_BINARY", &binary)
        .status()
        .map_err(|_| "installation failed")?;
    if !status.success() {
        return Err("installation failed");
    }
    let result = Command::new(prefix.join("bin/aiusage"))
        .arg("--version")
        .output()
        .map_err(|_| "installed version verification failed")?;
    if !result.status.success()
        || String::from_utf8_lossy(&result.stdout).trim() != format!("AIUsage {}", info.version)
    {
        return Err("installed version verification failed");
    }
    Ok(info.version.clone())
}
