use aiusage::cli;
use std::process::Command;

#[test]
fn home_without_environment_uses_account_database() {
    if std::env::var_os("AIUSAGE_TEST_HOME_FALLBACK").is_some() {
        let home = aiusage::config::home_dir();
        assert!(home.is_absolute());
        assert!(!home.as_os_str().is_empty());
        assert_eq!(
            aiusage::config::config_path(),
            home.join(".config/aiusage/config.toml")
        );
        assert_eq!(
            aiusage::updater::cache_path(),
            home.join(".cache/aiusage/latest.json")
        );
        return;
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "home_without_environment_uses_account_database"])
        .env("AIUSAGE_TEST_HOME_FALLBACK", "1")
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn dimensions_and_parser_contract() {
    assert_eq!(cli::dimensions("80X24"), Ok((80, 24)));
    assert_eq!(cli::dimensions(" 80 x 24 "), Ok((80, 24)));
    assert!(cli::dimensions("0x24").is_err());
    assert!(cli::dimensions("80x24x1").is_err());
    assert!(matches!(
        cli::parse(&["--snap".into()]),
        Ok(cli::Parsed::Args(_))
    ));
    assert!(cli::parse(&["--s".into()]).is_err());
    assert!(cli::parse(&["--unknown".into()]).is_err());
}

#[test]
fn help_version_and_error_contract() {
    let home = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_aiusage"))
            .args(args)
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path())
            .output()
            .unwrap()
    };
    let help = run(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert_eq!(String::from_utf8(help.stdout).unwrap(), cli::HELP);
    assert!(help.stderr.is_empty());

    let version = run(&["--version"]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        format!("AIUsage {}\n", aiusage::VERSION)
    );

    for (args, code, error) in [
        (&["--bad"][..], 2, "unrecognized arguments: --bad"),
        (&["--size"][..], 2, "argument --size: expected one argument"),
        (&[][..], 2, "aiusage requires an interactive terminal"),
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(code), "{args:?}");
        assert!(
            String::from_utf8(output.stderr).unwrap().contains(error),
            "{args:?}"
        );
    }

    for (value, error) in [
        ("0x24", "dimensions must be positive"),
        ("broken", "must be WIDTHxHEIGHT"),
    ] {
        let output = run(&["--demo", "--snapshot", "--size", value]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8(output.stderr).unwrap().contains(error));
    }
}

#[test]
fn demo_snapshot_is_isolated() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_aiusage"))
        .args(["--demo", "--snapshot", "--size", "80x24"])
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("PATH", home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for name in ["CODEX", "GROK", "MINIMAX", "QODER", "CODEBUDDY", "TRAECODE"] {
        assert!(text.contains(name));
    }
    assert!(text.contains("[演示]"));
    assert!(!home.path().join("aiusage/config.toml").exists());
}

#[test]
fn partial_paint_only_changed_rows() {
    assert_eq!(
        cli::paint(&["same".into()], ["same".into()].as_slice(), 24),
        ""
    );
    assert_eq!(
        cli::paint(&["new".into()], &["old".into()], 24),
        "\x1b[1;1H\x1b[2Knew"
    );
    assert_eq!(cli::paint(&[], &["old".into()], 24), "\x1b[1;1H\x1b[2K");
}
