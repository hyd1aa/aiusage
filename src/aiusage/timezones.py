import datetime as dt
import re


MINUTES_MIN = -12 * 60
MINUTES_MAX = 14 * 60
OFFSET_RE = re.compile(r"^UTC(?:(?P<sign>[+-])(?P<hour>\d{2})(?::(?P<minute>\d{2}))?)?$")
PRESETS = ("system", "UTC", "UTC+08", "UTC+09", "UTC-04", "UTC-05")


def parse_timezone(value: str):
    """Return None for the live system zone, or a validated fixed-offset zone."""
    if value == "system":
        return None
    match = OFFSET_RE.fullmatch(value)
    if not match:
        raise ValueError("invalid timezone")
    if not match.group("sign"):
        return dt.timezone.utc
    hour = int(match.group("hour"))
    minute = int(match.group("minute") or 0)
    total = hour * 60 + minute
    if minute >= 60 or total > (14 * 60 if match.group("sign") == "+" else 12 * 60):
        raise ValueError("timezone offset out of range")
    if total == 0:
        return dt.timezone.utc
    if match.group("sign") == "-":
        total = -total
    return dt.timezone(dt.timedelta(minutes=total))


def valid_timezone(value: str) -> bool:
    try:
        parse_timezone(value)
        return True
    except (TypeError, ValueError):
        return False


def convert(value: dt.datetime, setting: str) -> dt.datetime:
    if value.tzinfo is None:
        value = value.replace(tzinfo=dt.timezone.utc)
    zone = parse_timezone(setting)
    return value.astimezone() if zone is None else value.astimezone(zone)


def from_epoch(epoch: float, setting: str) -> dt.datetime:
    return convert(dt.datetime.fromtimestamp(epoch, dt.timezone.utc), setting)


def offset_label(value: dt.datetime) -> str:
    offset = value.utcoffset() or dt.timedelta(0)
    minutes = int(offset.total_seconds() // 60)
    if minutes == 0:
        return "UTC"
    sign = "+" if minutes > 0 else "-"
    minutes = abs(minutes)
    hours, remainder = divmod(minutes, 60)
    return f"UTC{sign}{hours:02d}" + (f":{remainder:02d}" if remainder else "")


def offset_setting(minutes: int) -> str:
    if not MINUTES_MIN <= minutes <= MINUTES_MAX:
        raise ValueError("timezone offset out of range")
    if minutes == 0:
        return "UTC"
    sign = "+" if minutes > 0 else "-"
    hours, remainder = divmod(abs(minutes), 60)
    return f"UTC{sign}{hours:02d}" + (f":{remainder:02d}" if remainder else "")


def offset_minutes(setting: str) -> int:
    zone = parse_timezone(setting)
    if zone is None:
        return 0
    return int((zone.utcoffset(None) or dt.timedelta()).total_seconds() // 60)
