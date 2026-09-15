use aiusage::cli;
use std::process::Command;

#[test]
fn home_without_environment_matches_python() {
    if std::env::var_os("AIUSAGE_TEST_HOME_FALLBACK").is_some() {
        let expected = Command::new("python3")
            .args(["-c", "import pathlib; print(pathlib.Path.home())"])
            .output()
            .unwrap();
        assert!(expected.status.success());
        assert_eq!(
            aiusage::config::home_dir().to_string_lossy(),
            String::from_utf8(expected.stdout).unwrap().trim_end()
        );
        assert_eq!(
            aiusage::config::config_path(),
            aiusage::config::home_dir().join(".config/aiusage/config.toml")
        );
        assert_eq!(
            aiusage::updater::cache_path(),
            aiusage::config::home_dir().join(".cache/aiusage/latest.json")
        );
        return;
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "home_without_environment_matches_python"])
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
fn help_version_and_errors_match_python() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["--help"],
        vec!["--help", "--s"],
        vec!["-hh"],
        vec!["-hX"],
        vec!["--help=X"],
        vec!["--version"],
        vec!["--bad"],
        vec!["--"],
        vec!["--", "--help"],
        vec!["--size", "-1"],
        vec!["--size", "-١"],
        vec!["--size", "-"],
        vec!["--size", "-.5"],
        vec!["--size", "-1."],
        vec!["--demo", "--snapshot", "--size", "-1"],
        vec![],
        vec!["--size"],
        vec!["--demo", "--snapshot", "--size", "0x24"],
        vec!["--demo", "--snapshot", "--size", "broken"],
    ] {
        let actual = Command::new(env!("CARGO_BIN_EXE_aiusage"))
            .args(&args)
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path())
            .output()
            .unwrap();
        let expected = Command::new("python3")
            .args(["-m", "aiusage.cli"])
            .args(&args)
            .env("PYTHONPATH", concat!(env!("CARGO_MANIFEST_DIR"), "/../src"))
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path())
            .output()
            .unwrap();
        assert_eq!(actual.status.code(), expected.status.code(), "{args:?}");
        assert_eq!(actual.stdout, expected.stdout, "stdout {args:?}");
        assert_eq!(actual.stderr, expected.stderr, "stderr {args:?}");
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
