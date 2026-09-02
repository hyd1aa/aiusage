from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class Availability(str, Enum):
    AVAILABLE = "available"
    NOT_INSTALLED = "not_installed"
    UNAVAILABLE = "unavailable"
    NOT_SUPPORTED = "not_supported"


@dataclass(frozen=True)
class RateLimitWindow:
    label: str
    remaining_percent: int
    reset_at: Optional[float] = None


@dataclass(frozen=True)
class ProviderUsage:
    key: str
    name: str
    availability: Availability
    windows: tuple[RateLimitWindow, ...] = field(default_factory=tuple)
    stale: bool = False
    error: Optional[str] = None

