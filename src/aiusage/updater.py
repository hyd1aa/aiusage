import json
import os
import re
import shutil
import subprocess
import tarfile
import tempfile
import time
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlparse


REPOSITORY = "hyd1aa/aiusage"
REPOSITORY_URL = f"https://github.com/{REPOSITORY}"
LATEST_API = f"https://api.github.com/repos/{REPOSITORY}/releases/latest"
CACHE_SECONDS = 6 * 60 * 60


@dataclass(frozen=True)
class ReleaseInfo:
    version: str
    title: str
    notes: str
    tarball_url: str


def cache_path() -> Path:
    base = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    return base / "aiusage" / "latest.json"


def _parse(payload) -> ReleaseInfo:
    if not isinstance(payload, dict) or payload.get("draft") or payload.get("prerelease"):
        raise ValueError("invalid stable release")
    tag = payload.get("tag_name")
    url = payload.get("tarball_url")
    if not isinstance(tag, str) or not re.fullmatch(r"v\d+\.\d+\.\d+", tag) or not isinstance(url, str):
        raise ValueError("malformed release response")
    host = (urlparse(url).hostname or "").lower()
    if host not in {"api.github.com", "github.com", "codeload.github.com"}:
        raise ValueError("untrusted release source")
    path = urlparse(url).path.lower()
    if "hyd1aa/aiusage" not in path and "repos/hyd1aa/aiusage" not in path:
        raise ValueError("release source is not the official repository")
    return ReleaseInfo(tag[1:], str(payload.get("name") or tag), str(payload.get("body") or ""), url)


def check_latest(timeout=2.0, opener=urllib.request.urlopen) -> ReleaseInfo:
    request = urllib.request.Request(
        LATEST_API,
        headers={"Accept": "application/vnd.github+json", "User-Agent": "AIUsage"},
    )
    with opener(request, timeout=timeout) as response:
        info = _parse(json.load(response))
    _save_cache(info)
    return info


def _save_cache(info: ReleaseInfo):
    target = cache_path()
    temporary = target.with_suffix(".tmp")
    try:
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        temporary.write_text(json.dumps({**info.__dict__, "checked_at": time.time()}), encoding="utf-8")
        os.chmod(temporary, 0o600)
        os.replace(temporary, target)
    except OSError:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass


def cached_latest(max_age=CACHE_SECONDS):
    try:
        payload = json.loads(cache_path().read_text(encoding="utf-8"))
        if time.time() - float(payload.pop("checked_at")) > max_age:
            return None
        return ReleaseInfo(**payload)
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError):
        return None


def version_tuple(value):
    try:
        return tuple(int(part) for part in value.split("."))
    except (AttributeError, ValueError):
        return ()


def is_newer(latest, current):
    return version_tuple(latest) > version_tuple(current)


def _safe_extract(archive: tarfile.TarFile, destination: Path):
    root = destination.resolve()
    members = archive.getmembers()
    for member in members:
        target = (destination / member.name).resolve()
        if root not in target.parents and target != root:
            raise ValueError("unsafe release archive")
        if member.issym() or member.islnk() or not (member.isfile() or member.isdir()):
            raise ValueError("release archive contains an unsafe entry")
    archive.extractall(destination, members=members)


def _source_version(source: Path) -> str:
    try:
        text = (source / "src" / "aiusage" / "__init__.py").read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        raise ValueError("release has no readable version source") from None
    match = re.search(r'^__version__\s*=\s*["\'](\d+\.\d+\.\d+)["\']\s*$', text, re.MULTILINE)
    if not match:
        raise ValueError("release has no valid version")
    return match.group(1)


def install_release(info: ReleaseInfo, current_version: str, prefix="/usr/local", runner=subprocess.run):
    if not is_newer(info.version, current_version):
        return False, "not newer"
    with tempfile.TemporaryDirectory(prefix="aiusage-update-") as directory:
        root = Path(directory)
        archive_path = root / "release.tar.gz"
        request = urllib.request.Request(info.tarball_url, headers={"User-Agent": "AIUsage"})
        with urllib.request.urlopen(request, timeout=15) as response, archive_path.open("wb") as output:
            shutil.copyfileobj(response, output)
        source = root / "source"
        source.mkdir()
        with tarfile.open(archive_path, "r:gz") as archive:
            _safe_extract(archive, source)
        roots = [item for item in source.iterdir() if item.is_dir()]
        if len(roots) != 1 or not (roots[0] / "install.sh").is_file():
            raise ValueError("invalid AIUsage release archive")
        if _source_version(roots[0]) != info.version:
            raise ValueError("release tag and source version do not match")
        env = os.environ.copy()
        env["PREFIX"] = prefix
        command = [str(roots[0] / "install.sh")]
        if prefix == "/usr/local" and os.geteuid() != 0:
            command.insert(0, "sudo")
        runner(command, cwd=roots[0], env=env, check=True)
        launcher = Path(prefix) / "bin" / "aiusage"
        result = runner([str(launcher), "--version"], text=True, capture_output=True, check=True)
        if result.stdout.strip() != f"AIUsage {info.version}":
            raise RuntimeError("installed version verification failed")
    return True, info.version
