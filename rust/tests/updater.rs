use aiusage::updater::*;
use serde_json::json;
use sha2::{Digest, Sha256};
#[test]
fn official_stable_release_only() {
    let valid = json!({"tag_name":"v0.2.3","tarball_url":"https://api.github.com/repos/hyd1aa/aiusage/tarball/v0.2.3"});
    assert_eq!(parse(&valid).unwrap().version, "0.2.3");
    assert!(is_newer("0.2.3", "0.2.2"));
    assert!(!is_newer("0.2.2", "0.2.2"));
    for field in ["draft", "prerelease"] {
        let mut value = valid.clone();
        value[field] = json!(true);
        assert!(parse(&value).is_err());
    }
    for url in [
        "https://evil.example/hyd1aa/aiusage/x",
        "http://api.github.com/repos/hyd1aa/aiusage/x",
        "https://github.com/other/hyd1aa/aiusage/x",
        "https://github.com/hyd1aa/aiusage-evil/x",
    ] {
        let mut value = valid.clone();
        value["tarball_url"] = json!(url);
        assert!(parse(&value).is_err());
    }
    for tag in ["v9", "v1.0.0-rc1", "1.0.0"] {
        let mut value = valid.clone();
        value["tag_name"] = json!(tag);
        assert!(parse(&value).is_err());
    }
}
#[test]
fn cache_compatible_private_and_expires() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("cache/latest.json");
    let info=parse(&json!({"tag_name":"v0.2.3","tarball_url":"https://github.com/hyd1aa/aiusage/archive/v0.2.3.tar.gz"})).unwrap();
    assert!(save_cache(&info, &path, 1000.0));
    assert_eq!(cached_at(&path, 1001.0, CACHE_SECONDS), Some(info));
    assert!(cached_at(&path, 1000.0 + CACHE_SECONDS + 1.0, CACHE_SECONDS).is_none());
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(!std::fs::read_to_string(path).unwrap().contains("assets"));
}
#[test]
fn missing_binary_has_no_source_fallback() {
    let info=parse(&json!({"tag_name":"v0.2.3","tarball_url":"https://github.com/hyd1aa/aiusage/archive/v0.2.3.tar.gz"})).unwrap();
    assert!(binary_asset(&info, "linux-arm64").is_err());
}
#[test]
fn asset_requires_exact_target_origin_and_digest() {
    let bytes = b"synthetic binary";
    let name = "aiusage-v0.2.3-linux-arm64";
    let mut info=parse(&json!({"tag_name":"v0.2.3","tarball_url":"https://github.com/hyd1aa/aiusage/archive/v0.2.3.tar.gz",
        "assets":[{"name":name,"browser_download_url":format!("{REPOSITORY_URL}/releases/download/v0.2.3/{name}"),"digest":format!("sha256:{:x}",Sha256::digest(bytes))}]})).unwrap();
    let asset = binary_asset(&info, "linux-arm64").unwrap();
    assert!(verify_digest(bytes, asset));
    assert!(!verify_digest(b"tampered", asset));
    assert!(binary_asset(&info, "linux-amd64").is_err());
    info.assets[0].browser_download_url = "https://evil.example/binary".into();
    assert!(binary_asset(&info, "linux-arm64").is_err());
}
