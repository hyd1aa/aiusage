"""Deterministic demo fixtures, isolated from all real provider readers."""

import time

from .models import Availability, ProviderUsage, RateLimitWindow
from .providers import REGISTRY


VALUES = {
    "codex": (("5h", 83, 2.2), ("Week", 61, 74)),
    "grok": (("Cycle", 72, 51),),
    "antigravity": (("Daily", 64, 10),),
    "claude": (("5h", 48, 3.5),),
    "gemini": (("Daily", 91, 8),),
    "deepseek": (("Daily", 66, 11),),
    "kimi": (("Monthly", 37, 240),),
    "glm": (("Daily", 79, 13),),
    "zai": (("Cycle", 55, 36),),
}


def demo_usage(key: str) -> ProviderUsage:
    # Relative reset times stay plausible; percentages and window labels are
    # fixed for reproducible layout and screenshot tests.
    now = time.time()
    adapter = REGISTRY[key]
    windows = tuple(
        RateLimitWindow(label, remaining, now + hours * 3600)
        for label, remaining, hours in VALUES[key]
    )
    return ProviderUsage(key, adapter.name, Availability.AVAILABLE, windows)
