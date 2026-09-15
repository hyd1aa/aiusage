use std::{env, fs};

fn main() {
    // During migration the untouched reference remains the version authority.
    // Compile it into the executable; no Python/source dependency at runtime.
    let source = "../src/aiusage/__init__.py";
    println!("cargo:rerun-if-changed={source}");
    let text = fs::read_to_string(source).expect("reference version source");
    let version = text
        .lines()
        .find_map(|line| {
            line.strip_prefix("__version__ = ")
                .map(|value| value.trim_matches('"'))
        })
        .expect("reference version");
    assert_eq!(
        version,
        env::var("CARGO_PKG_VERSION").unwrap(),
        "Cargo metadata drift"
    );
    println!("cargo:rustc-env=AIUSAGE_VERSION={version}");
}
