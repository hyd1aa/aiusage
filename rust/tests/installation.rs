use aiusage::updater::{self, Asset, ReleaseInfo};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    process::Command,
};

fn metadata(path: &Path) -> (u32, u32, u32) {
    let value = fs::metadata(path).unwrap();
    (value.mode() & 0o7777, value.uid(), value.gid())
}
fn install(prefix: &Path) -> std::process::Output {
    Command::new("sh")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
        .env("PREFIX", prefix)
        .env("AIUSAGE_BINARY", env!("CARGO_BIN_EXE_aiusage"))
        .output()
        .unwrap()
}
fn prepare(root: &Path, bin: u32, lib: u32) -> std::path::PathBuf {
    let prefix = root.join("prefix");
    fs::create_dir_all(prefix.join("bin")).unwrap();
    fs::create_dir(prefix.join("lib")).unwrap();
    fs::set_permissions(prefix.join("bin"), fs::Permissions::from_mode(bin)).unwrap();
    fs::set_permissions(prefix.join("lib"), fs::Permissions::from_mode(lib)).unwrap();
    prefix
}
#[test]
fn lifecycle_preserves_shared_directory_metadata_and_third_party_ai() {
    for (bin, lib) in [
        (0o775, 0o2775),
        (0o770, 0o2775),
        (0o700, 0o700),
        (0o1775, 0o2775),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let prefix = prepare(temp.path(), bin, lib);
        let foreign = prefix.join("bin/ai");
        fs::write(&foreign, "third-party-ai").unwrap();
        let before = (metadata(&prefix.join("bin")), metadata(&prefix.join("lib")));
        for _ in 0..2 {
            let result = install(&prefix);
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(
                (metadata(&prefix.join("bin")), metadata(&prefix.join("lib"))),
                before
            );
            assert_eq!(fs::read_to_string(&foreign).unwrap(), "third-party-ai");
            let version = Command::new(prefix.join("bin/aiusage"))
                .arg("--version")
                .output()
                .unwrap();
            assert_eq!(
                String::from_utf8_lossy(&version.stdout).trim(),
                format!("AIUsage {}", aiusage::VERSION)
            );
        }
        let result = Command::new("sh")
            .arg(prefix.join("lib/aiusage-uninstall.sh"))
            .env("PREFIX", &prefix)
            .output()
            .unwrap();
        assert!(result.status.success());
        assert_eq!(
            (metadata(&prefix.join("bin")), metadata(&prefix.join("lib"))),
            before
        );
        assert!(foreign.exists());
        assert!(!prefix.join("lib/aiusage").exists());
    }
}
#[test]
fn fresh_directories_and_reserved_paths() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("fresh");
    assert!(install(&prefix).status.success());
    assert_eq!(metadata(&prefix.join("bin")).0, 0o755);
    assert_eq!(metadata(&prefix.join("lib")).0, 0o755);
    for relative in [
        "bin",
        "lib",
        "bin/aiusage",
        "lib/aiusage",
        "lib/aiusage-uninstall.sh",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let prefix = temp.path().join("prefix");
        let target = prefix.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "foreign").unwrap();
        assert!(!install(&prefix).status.success(), "{relative}");
        assert_eq!(fs::read_to_string(target).unwrap(), "foreign");
    }
}
fn release(bytes: &[u8]) -> ReleaseInfo {
    let name = format!(
        "aiusage-v{}-{}",
        aiusage::VERSION,
        updater::target_name().unwrap()
    );
    ReleaseInfo {
        version: aiusage::VERSION.into(),
        title: String::new(),
        notes: String::new(),
        tarball_url: format!(
            "https://github.com/hyd1aa/aiusage/archive/v{}.tar.gz",
            aiusage::VERSION
        ),
        assets: vec![Asset {
            name: name.clone(),
            browser_download_url: format!(
                "{}/releases/download/v{}/{name}",
                updater::REPOSITORY_URL,
                aiusage::VERSION
            ),
            digest: Some(format!("sha256:{:x}", Sha256::digest(bytes))),
        }],
    }
}
#[test]
fn updater_uses_verified_binary_and_preserves_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = prepare(temp.path(), 0o775, 0o2775);
    let before = (metadata(&prefix.join("bin")), metadata(&prefix.join("lib")));
    let bytes = fs::read(env!("CARGO_BIN_EXE_aiusage")).unwrap();
    let info = release(&bytes);
    assert_eq!(
        updater::install_release_with(&info, "0.1.0", &prefix, |_| Ok(bytes)),
        Ok(aiusage::VERSION.into())
    );
    assert_eq!(
        (metadata(&prefix.join("bin")), metadata(&prefix.join("lib"))),
        before
    );
}
#[test]
fn tampered_update_does_not_install() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("prefix");
    let info = release(b"expected");
    assert!(
        updater::install_release_with(&info, "0.1.0", &prefix, |_| Ok(b"tampered".to_vec()))
            .is_err()
    );
    assert!(!prefix.exists());
    assert!(
        updater::install_release_with(&info, aiusage::VERSION, &prefix, |_| panic!(
            "download on no-op"
        ))
        .is_err()
    );
}

#[test]
fn publication_failure_restores_previous_owned_installation() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = prepare(temp.path(), 0o775, 0o2775);
    assert!(install(&prefix).status.success());
    let marker = prefix.join("lib/aiusage/previous-installation");
    fs::write(&marker, "preserved on rollback").unwrap();
    let tools = temp.path().join("tools");
    fs::create_dir(&tools).unwrap();
    let mv = tools.join("mv");
    fs::write(&mv,"#!/bin/sh\nfor arg in \"$@\"; do\ncase \"$arg\" in */uninstall.sh) exit 1;; esac\ndone\nexec /bin/mv \"$@\"\n").unwrap();
    fs::set_permissions(&mv, fs::Permissions::from_mode(0o700)).unwrap();
    let path = std::env::join_paths(std::iter::once(tools).chain(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    )))
    .unwrap();
    let result = Command::new("sh")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
        .env("PREFIX", &prefix)
        .env("AIUSAGE_BINARY", env!("CARGO_BIN_EXE_aiusage"))
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(marker).unwrap(), "preserved on rollback");
    assert!(Command::new(prefix.join("bin/aiusage"))
        .arg("--version")
        .status()
        .unwrap()
        .success());
    assert!(prefix.join("lib/aiusage-uninstall.sh").exists());
}

#[test]
fn release_version_mismatch_fails_before_installation() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("prefix");
    let bytes = fs::read(env!("CARGO_BIN_EXE_aiusage")).unwrap();
    let mut info = release(&bytes);
    info.version = "9.0.0".into();
    let name = format!("aiusage-v9.0.0-{}", updater::target_name().unwrap());
    info.assets[0].name = name.clone();
    info.assets[0].browser_download_url = format!(
        "{}/releases/download/v9.0.0/{name}",
        updater::REPOSITORY_URL
    );
    assert_eq!(
        updater::install_release_with(&info, "0.1.0", &prefix, |_| Ok(bytes)),
        Err("release version verification failed")
    );
    assert!(!prefix.exists());
}
