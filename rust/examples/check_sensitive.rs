use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, process::Command};

fn command(args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .output()
        .expect("git is required");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

fn prefixed_run(bytes: &[u8], prefix: &[u8], minimum: usize, allowed: fn(u8) -> bool) -> bool {
    bytes
        .windows(prefix.len())
        .enumerate()
        .any(|(index, window)| {
            if window != prefix {
                return false;
            }
            bytes[index + prefix.len()..]
                .iter()
                .take_while(|byte| allowed(**byte))
                .count()
                >= minimum
        })
}

fn private_key(bytes: &[u8]) -> bool {
    let mut begin = b"-----".to_vec();
    begin.extend_from_slice(b"BEGIN ");
    let mut end = b"PRIVATE ".to_vec();
    end.extend_from_slice(b"KEY-----");
    bytes
        .windows(begin.len())
        .enumerate()
        .any(|(index, window)| {
            if window != begin {
                return false;
            }
            let tail = &bytes[index + begin.len()..];
            tail.windows(end.len())
                .enumerate()
                .any(|(end_index, value)| {
                    value == end
                        && tail[..end_index]
                            .iter()
                            .all(|byte| byte.is_ascii_uppercase() || *byte == b' ')
                })
        })
}

fn ipv4(bytes: &[u8]) -> bool {
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_digit()
            || start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.')
        {
            continue;
        }
        let mut index = start;
        let mut groups = 0;
        while groups < 4 {
            let begin = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() && index - begin < 3 {
                index += 1;
            }
            if index == begin || index < bytes.len() && bytes[index].is_ascii_digit() {
                break;
            }
            groups += 1;
            if groups < 4 {
                if bytes.get(index) != Some(&b'.') {
                    break;
                }
                index += 1;
            }
        }
        if groups == 4
            && bytes
                .get(index)
                .is_none_or(|byte| !byte.is_ascii_digit() && *byte != b'.')
        {
            return true;
        }
    }
    false
}

fn machine_path(bytes: &[u8]) -> bool {
    for parent in ["root", "home", "opt"] {
        let prefix = format!("/{parent}/");
        for (index, window) in bytes.windows(prefix.len()).enumerate() {
            if window != prefix.as_bytes() {
                continue;
            }
            let tail = &bytes[index + prefix.len()..];
            let length = tail
                .iter()
                .take_while(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-')
                })
                .count();
            if length > 0 && tail.get(length) == Some(&b'/') {
                return true;
            }
        }
    }
    false
}

fn jwt(bytes: &[u8]) -> bool {
    let allowed = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-');
    for (index, window) in bytes.windows(3).enumerate() {
        if window != b"eyJ" {
            continue;
        }
        let mut rest = &bytes[index..];
        let mut valid = true;
        for part in 0..3 {
            let length = rest.iter().take_while(|byte| allowed(**byte)).count();
            if length < 10 {
                valid = false;
                break;
            }
            rest = &rest[length..];
            if part < 2 {
                if rest.first() != Some(&b'.') {
                    valid = false;
                    break;
                }
                rest = &rest[1..];
            }
        }
        if valid {
            return true;
        }
    }
    false
}

fn findings(content: &[u8]) -> Vec<&'static str> {
    let alnum_underscore = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let key_chars = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-');
    let slack_chars = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'-';
    let mut aws = b"AK".to_vec();
    aws.extend_from_slice(b"IA");
    let mut openai = b"sk".to_vec();
    openai.push(b'-');
    let mut authorization = b"authorization".to_vec();
    authorization.push(b':');
    let mut bearer = b"bear".to_vec();
    bearer.extend_from_slice(b"er ");
    let mut basic = b"bas".to_vec();
    basic.extend_from_slice(b"ic ");
    let lower = content
        .iter()
        .map(u8::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let mut result = vec![];
    if private_key(content) {
        result.push("private key");
    }
    if prefixed_run(content, &aws, 16, |byte| {
        byte.is_ascii_uppercase() || byte.is_ascii_digit()
    }) {
        result.push("AWS access key");
    }
    if b"pousr".iter().any(|kind| {
        let mut prefix = b"gh".to_vec();
        prefix.extend_from_slice(&[*kind, b'_']);
        prefixed_run(content, &prefix, 20, alnum_underscore)
    }) {
        result.push("GitHub token");
    }
    if prefixed_run(content, &openai, 20, key_chars) {
        result.push("OpenAI-style key");
    }
    if b"baprs".iter().any(|kind| {
        let mut prefix = b"xox".to_vec();
        prefix.extend_from_slice(&[*kind, b'-']);
        prefixed_run(content, &prefix, 10, slack_chars)
    }) {
        result.push("Slack token");
    }
    if jwt(content) {
        result.push("JWT");
    }
    if ipv4(content) {
        result.push("IPv4 address");
    }
    if machine_path(content) {
        result.push("machine-specific path");
    }
    if contains(&lower, &authorization) && (contains(&lower, &bearer) || contains(&lower, &basic)) {
        result.push("authorization value");
    }
    result
}

fn main() {
    let objects = String::from_utf8(command(&["rev-list", "--objects", "--all"])).unwrap();
    let mut checked = HashSet::new();
    let mut failures = vec![];
    for row in objects.lines() {
        let mut fields = row.splitn(2, ' ');
        let object = fields.next().unwrap();
        let path = fields.next().unwrap_or(object);
        if !checked.insert((object.to_string(), path.to_string()))
            || command(&["cat-file", "-t", object]) != b"blob\n"
        {
            continue;
        }
        let content = command(&["cat-file", "-p", object]);
        let digest = format!("{:x}", Sha256::digest(&content));
        for label in findings(&content) {
            let reviewed = path == "rust/tests/compatibility.rs"
                && label == "IPv4 address"
                && digest == "11f07332664075650d69e754a3123debdf3537e733d695a5a7322259bb608bf4";
            if !reviewed {
                failures.push(format!("{path}: {label}"));
            }
        }
    }
    for raw_path in command(&["ls-files", "-z"])
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = String::from_utf8_lossy(raw_path);
        let Ok(content) = fs::read(path.as_ref()) else {
            continue;
        };
        for label in findings(&content) {
            failures.push(format!("{path} (working tree): {label}"));
        }
    }
    if failures.is_empty() {
        println!("Sensitive-pattern check passed for all reachable Git blobs.");
    } else {
        eprintln!("Sensitive-pattern check failed:\n{}", failures.join("\n"));
        std::process::exit(1);
    }
}
