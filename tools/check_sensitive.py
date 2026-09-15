#!/usr/bin/env python3
"""Offline, high-confidence sensitive-data check for tracked Git content."""

import re
import hashlib
import subprocess
import sys


PATTERNS = {
    "private key": re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    "AWS access key": re.compile(rb"AKIA[0-9A-Z]{16}"),
    "GitHub token": re.compile(rb"gh[pousr]_[A-Za-z0-9_]{20,}"),
    "OpenAI-style key": re.compile(rb"\bsk-[A-Za-z0-9_-]{20,}"),
    "Slack token": re.compile(rb"xox[baprs]-[A-Za-z0-9-]{10,}"),
    "JWT": re.compile(rb"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"),
    "IPv4 address": re.compile(rb"(?<![\d.])(?:\d{1,3}\.){3}\d{1,3}(?![\d.])"),
    "machine-specific path": re.compile(rb"/(?:root|home|opt)/[A-Za-z0-9_.-]+/"),
    "authorization value": re.compile(rb"(?i)authorization\s*:\s*(?:bearer|basic)\s+[A-Za-z0-9+/_.=-]{8,}"),
}


def reviewed_fixture_match(path, content, label):
    # A historical Rust version-tuple fixture contains a four-component
    # version, not an address. Scope this exception to that exact reviewed
    # blob and this one pattern; all other secret checks still apply.
    return (
        path == "rust/tests/compatibility.rs"
        and label == "IPv4 address"
        and hashlib.sha256(content).hexdigest()
        == "11f07332664075650d69e754a3123debdf3537e733d695a5a7322259bb608bf4"
    )


def git_objects():
    rows = subprocess.check_output(["git", "rev-list", "--objects", "--all"]).splitlines()
    for row in rows:
        oid, *path = row.split(b" ", 1)
        kind = subprocess.check_output(["git", "cat-file", "-t", oid]).strip()
        if kind != b"blob":
            continue
        content = subprocess.check_output(["git", "cat-file", "-p", oid])
        yield (path[0].decode("utf-8", "replace") if path else oid.decode()), content


def main():
    findings = []
    for path, content in git_objects():
        for label, pattern in PATTERNS.items():
            if pattern.search(content) and not reviewed_fixture_match(path, content, label):
                findings.append(f"{path}: {label}")
    if findings:
        print("Sensitive-pattern check failed:", file=sys.stderr)
        print("\n".join(findings), file=sys.stderr)
        return 1
    print("Sensitive-pattern check passed for all reachable Git blobs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
