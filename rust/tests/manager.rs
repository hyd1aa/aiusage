use aiusage::{
    config::Config,
    manager::{Actions, Manager},
    providers::{classify_discovery, DiscoveryResult},
    updater::ReleaseInfo,
};
use std::{collections::HashMap, io::Cursor, time::Duration};

#[derive(Default)]
struct Fake {
    launches: Vec<bool>,
    installs: usize,
    uninstalls: Vec<bool>,
}
fn info() -> ReleaseInfo {
    ReleaseInfo {
        version: "9.0.0".into(),
        title: "fixture".into(),
        notes: String::new(),
        tarball_url: "https://api.github.com/repos/hyd1aa/aiusage/tarball/v9.0.0".into(),
        assets: vec![],
    }
}
impl Actions for Fake {
    fn latest(&mut self, _: Duration) -> Result<ReleaseInfo, String> {
        Ok(info())
    }
    fn discover(&mut self) -> HashMap<String, DiscoveryResult> {
        aiusage::PROVIDERS
            .iter()
            .map(|(k, _)| {
                (
                    k.to_string(),
                    classify_discovery(false, matches!(*k, "codex" | "grok"), false),
                )
            })
            .collect()
    }
    fn diagnostics(&mut self, _: &Config, _: bool) -> Vec<aiusage::diagnostics::Row> {
        vec![("Codex".into(), true, "readable".into())]
    }
    fn launch(&mut self, demo: bool) {
        self.launches.push(demo);
    }
    fn install(&mut self, info: &ReleaseInfo) -> Result<String, String> {
        self.installs += 1;
        Ok(info.version.clone())
    }
    fn uninstall(&mut self, remove: bool) -> Result<(), String> {
        self.uninstalls.push(remove);
        Ok(())
    }
}
fn manager(input: &str) -> Manager<Cursor<Vec<u8>>, Vec<u8>, Fake> {
    Manager::new(
        Config::default(),
        Cursor::new(input.as_bytes().to_vec()),
        vec![],
        Fake::default(),
    )
}
#[test]
fn dashboard_and_demo_return_to_menu() {
    let mut manager = manager("1\n2\n0\n");
    manager.run().unwrap();
    assert_eq!(manager.actions.launches, vec![false, true]);
}
#[test]
fn settings_share_config_and_persist() {
    let temp = tempfile::tempdir().unwrap();
    let mut manager = manager("2\n4\n3\n6\n0\n");
    manager.config_path = temp.path().join("config.toml");
    manager.settings().unwrap();
    let cfg = aiusage::config::load(&manager.config_path);
    assert_eq!(cfg.theme, "green");
    assert_eq!(cfg.timezone, "UTC+08");
    assert!(!cfg.auto_discover);
}
#[test]
fn provider_cancel_save_reorder_and_disable() {
    let temp = tempfile::tempdir().unwrap();
    let mut menu = manager("3\n0\n");
    menu.config_path = temp.path().join("config.toml");
    menu.provider_menu().unwrap();
    assert_eq!(menu.cfg.real_providers, vec!["codex", "grok"]);
    menu.input = Cursor::new(b"3\nu3\n2\ns\n".to_vec());
    menu.provider_menu().unwrap();
    assert_eq!(menu.cfg.real_providers, vec!["codex", "minimax"]);
    assert!(menu.cfg.disabled_providers.contains(&"grok".into()));
    assert_eq!(aiusage::config::load(&menu.config_path), menu.cfg);
}
#[test]
fn update_needs_confirmation() {
    let mut menu = manager("n\n");
    menu.update_menu().unwrap();
    assert_eq!(menu.actions.installs, 0);
    menu.input = Cursor::new(b"y\n".to_vec());
    menu.update_menu().unwrap();
    assert_eq!(menu.actions.installs, 1);
}
#[test]
fn uninstall_needs_choice_and_confirmation() {
    let mut menu = manager("0\n");
    assert!(!menu.uninstall_menu().unwrap());
    assert!(menu.actions.uninstalls.is_empty());
    menu.input = Cursor::new(b"1\nn\n".to_vec());
    assert!(!menu.uninstall_menu().unwrap());
    menu.input = Cursor::new(b"1\ny\n".to_vec());
    assert!(menu.uninstall_menu().unwrap());
    menu.input = Cursor::new(b"2\nyes\n".to_vec());
    assert!(menu.uninstall_menu().unwrap());
    assert_eq!(menu.actions.uninstalls, vec![false, true]);
}
#[test]
fn diagnostics_safe_output() {
    let mut menu = manager("\n");
    menu.diagnostics().unwrap();
    let output = String::from_utf8(menu.output).unwrap();
    assert!(output.contains("Codex: 可读取"));
    for forbidden in ["token", "cookie", "authorization", "password"] {
        assert!(!output.to_lowercase().contains(forbidden));
    }
}

#[test]
fn manager_rechecks_terminal_width_before_each_write() {
    let mut menu = manager("");
    menu.width = 80;
    menu.live_width = Some(|| 12);
    menu.write("abcdefghijklmnopqrst", false).unwrap();
    menu.live_width = Some(|| 16);
    menu.write("abcdefghijklmnopqrst", false).unwrap();
    assert_eq!(
        String::from_utf8(menu.output).unwrap(),
        "abcdefghijkl\nabcdefghijklmnop\n"
    );
}

#[test]
fn real_diagnostics_are_sanitized_with_isolated_empty_provider_environment() {
    if std::env::var_os("AIUSAGE_DIAGNOSTIC_CHILD").is_some() {
        let rows = aiusage::diagnostics::collect(&Config::default(), Some(false));
        let text = format!("{rows:?}");
        assert!(!text.contains("fixture-private-marker"));
        assert!(rows
            .iter()
            .any(|(name, ok, detail)| name == "Rust" && *ok && detail == "native binary"));
        assert!(rows
            .iter()
            .any(|(name, ok, detail)| name == "Codex usage" && !ok && detail == "unavailable"));
        assert!(rows.iter().any(|(name, ok, _)| name == "GitHub" && !ok));
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "real_diagnostics_are_sanitized_with_isolated_empty_provider_environment",
        ])
        .env("AIUSAGE_DIAGNOSTIC_CHILD", "1")
        .env("HOME", home.path())
        .env("PATH", home.path())
        .env("LC_ALL", "fixture-private-marker.UTF-8")
        .env_remove("CODEX_API_KEY")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
