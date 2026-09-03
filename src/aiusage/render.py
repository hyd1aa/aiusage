import datetime as dt
import re
import unicodedata

from .i18n import tr
from .models import Availability, ProviderUsage
from .timezones import convert, from_epoch, offset_label

ANSI = re.compile(r"\x1b\[[0-9;]*m")
MIN_CELL_WIDTH = 24
CELL_GAP = 2
RESET_STYLE = "\x1b[0m"
THEME_STYLE = {
    "white": {"normal": "\x1b[37m", "strong": "\x1b[1;97m", "muted": "\x1b[90m"},
    "green": {"normal": "\x1b[32m", "strong": "\x1b[1;92m", "muted": "\x1b[32m"},
}


def visible_len(value):
    return sum(_char_width(char) for char in ANSI.sub("", value))


def _char_width(char):
    if unicodedata.combining(char) or unicodedata.category(char).startswith("C"):
        return 0
    return 2 if unicodedata.east_asian_width(char) in ("W", "F") else 1


def _fit(value, width):
    result = []
    used = 0
    for char in value:
        size = _char_width(char)
        if used + size > max(0, width):
            break
        result.append(char)
        used += size
    return "".join(result)


def _pad(value, width):
    value = _fit(value, width)
    return value + " " * max(0, width - visible_len(value))


def reset_text(epoch, language="en", timezone="system"):
    """Convert and format a reset epoch in the selected display timezone."""
    if not isinstance(epoch, (int, float)):
        return "?"
    value = from_epoch(epoch, timezone)
    zone = offset_label(value)
    if language == "zh":
        return f"{value.month}月{value.day:02d}日 {value:%H:%M} {zone}"
    return f"{value:%b %d %H:%M} {zone}"


def system_text(language="en", timezone="system", now=None):
    value = convert(now or dt.datetime.now(dt.timezone.utc), timezone)
    zone = offset_label(value)
    return _label_value(tr(language, "system"), f"{value:%Y-%m-%d %H:%M:%S} {zone}", language)


def _label_value(label, value, language):
    return f"{label}：{value}" if language == "zh" else f"{label}: {value}"


def column_count(count, width):
    if count <= 2:
        return 1
    capacity = max(1, min(3, (width + CELL_GAP) // (MIN_CELL_WIDTH + CELL_GAP)))
    if count == 4 and capacity >= 2:
        return 2
    return min(count, capacity)


def _provider_lines(provider: ProviderUsage, width: int, language: str, timezone: str):
    suffix = f"  ! {tr(language, 'stale')}" if provider.stale else ""
    lines = [provider.name.upper() + suffix]
    if provider.availability != Availability.AVAILABLE or not provider.windows:
        lines.append(tr(language, provider.availability.value))
        return [_fit(line, width) for line in lines]

    bar_width = max(3, min(10, width - 17))
    for window in provider.windows[:2]:
        remaining = max(0, min(100, int(window.remaining_percent)))
        fill = round(remaining * bar_width / 100)
        bar = "█" * fill + "░" * (bar_width - fill)
        label = "Weekly" if window.label == "Week" and language == "en" else window.label
        lines.append(f"{_fit(label, 6):<6} {bar} {remaining:>3}% {tr(language, 'left')}")
        reset_value = reset_text(window.reset_at, language, timezone)
        reset_line = _label_value(tr(language, "reset"), reset_value, language)
        # In narrow grids preserve the complete instant and UTC offset; the
        # repeated Reset label is less important than an unambiguous timestamp.
        lines.append(reset_line if visible_len(reset_line) <= width else reset_value)
    return [_fit(line, width) for line in lines]


def _top_border(width, title):
    inner = max(0, width - 2)
    token = _fit(f" {title} ", inner)
    remaining = max(0, inner - visible_len(token))
    left = remaining // 2
    return "┌" + "─" * left + token + "─" * (remaining - left) + ("┐" if width > 1 else "")


def _outer_line(value, width):
    if width < 2:
        return _fit(value, width)
    inner = width - 2
    return "│" + _pad(value, inner) + "│"


def _position(lines, terminal_width, terminal_height, position):
    block_width = max((visible_len(line) for line in lines), default=0)
    block_height = len(lines)
    vertical, horizontal = ("center", "center") if position == "center" else position.split("-", 1)
    left = {
        "left": 0,
        "center": max(0, (terminal_width - block_width) // 2),
        "right": max(0, terminal_width - block_width),
    }[horizontal]
    top = {
        "top": 0,
        "center": max(0, (terminal_height - block_height) // 2),
        "bottom": max(0, terminal_height - block_height),
    }[vertical]
    return [""] * top + [_fit(" " * left + line, terminal_width) for line in lines]


def _style(lines, kinds, theme, color):
    if not color:
        return lines
    palette = THEME_STYLE.get(theme, THEME_STYLE["white"])
    return [
        f"{palette.get(kind, palette['normal'])}{line}{RESET_STYLE}" if line else line
        for line, kind in zip(lines, kinds)
    ]


def dashboard(
    width,
    height,
    providers,
    updated,
    language="en",
    position="center",
    demo=False,
    theme="white",
    color=False,
    timezone="system",
):
    width, height = max(1, width), max(1, height)
    title = f"AI USAGE [{tr(language, 'demo')}]" if demo else "AI USAGE"
    if width < 4 or height < 3:
        return _position([_fit(title, width)][:height], width, height, position)

    help_text = tr(language, "help")
    compact_help = tr(language, "help_compact")
    updated_at = convert(updated, timezone) if updated else None
    stamp = updated_at.strftime("%H:%M:%S") if updated_at else "--:--:--"
    update_text = _label_value(tr(language, "updated"), stamp, language)

    content_capacity = max(1, width - 4)
    columns = column_count(len(providers), content_capacity)
    preferred_cell = {1: 40, 2: 27, 3: 24}[columns]
    cell_width = min(preferred_cell, max(1, (content_capacity - CELL_GAP * (columns - 1)) // columns))
    grid_width = cell_width * columns + CELL_GAP * (columns - 1)
    desired_outer = max(grid_width + 4, visible_len(compact_help) + 4, visible_len(system_text(language, timezone)) + 4)
    outer_width = min(width, desired_outer)
    inner_width = outer_width - 2
    content_width = max(1, inner_width - 2)
    cell_width = min(cell_width, max(1, (content_width - CELL_GAP * (columns - 1)) // columns))
    grid_width = cell_width * columns + CELL_GAP * (columns - 1)

    blocks = [_provider_lines(provider, cell_width, language, timezone) for provider in providers]

    content = [""]
    content_kinds = ["normal"]
    rows = (len(blocks) + columns - 1) // columns
    for row_index in range(rows):
        group = blocks[row_index * columns:(row_index + 1) * columns]
        row_height = max((len(block) for block in group), default=1)
        group = [block + [""] * (row_height - len(block)) for block in group]
        group_width = len(group) * cell_width + max(0, len(group) - 1) * CELL_GAP
        grid_left = max(0, (content_width - group_width) // 2)
        for line_index in range(row_height):
            value = (" " * CELL_GAP).join(_pad(block[line_index], cell_width) for block in group)
            content.append(" " + " " * grid_left + value)
            content_kinds.append("strong" if line_index == 0 else "normal")
        if row_index < rows - 1:
            content.append("")
            content_kinds.append("normal")

    chosen_help = help_text if visible_len(help_text) <= content_width - 1 else compact_help
    content.extend(["", " " + system_text(language, timezone), " " + update_text, "", " " + chosen_help, ""])
    content_kinds.extend(["normal", "normal", "muted", "normal", "muted", "normal"])

    max_content_lines = max(0, height - 2)
    if len(content) > max_content_lines:
        content = content[:max_content_lines]
        content_kinds = content_kinds[:max_content_lines]

    raw_lines = [_top_border(outer_width, title)]
    kinds = ["strong"]
    for value, kind in zip(content, content_kinds):
        raw_lines.append(_outer_line(_fit(value, inner_width), outer_width))
        kinds.append(kind)
    raw_lines.append("└" + "─" * (outer_width - 2) + "┘")
    kinds.append("normal")

    positioned = _position(raw_lines[:height], width, height, position)
    top_padding = len(positioned) - len(raw_lines[:height])
    styled = _style(positioned[top_padding:], kinds[: len(positioned) - top_padding], theme, color)
    return positioned[:top_padding] + styled


def selector(width, height, registry, enabled, cursor, language, theme="white", color=False):
    box_width = min(58, max(30, width - 4))
    inner = box_width - 2
    lines = ["┌" + "─" * inner + "┐"]
    title = tr(language, "providers")
    lines.append("│ " + _pad(title, inner - 2) + " │")
    for index, (key, adapter) in enumerate(registry.items()):
        mark = "x" if key in enabled else " "
        pointer = "›" if index == cursor else " "
        order = enabled.index(key) + 1 if key in enabled else 0
        value = f"{pointer} [{mark}] {adapter.name}" + (f"  {order}" if order else "")
        lines.append("│ " + _pad(value, inner - 2) + " │")
    help_value = _fit(tr(language, "select_help"), inner - 2)
    lines.append("│ " + _pad(help_value, inner - 2) + " │")
    lines.append("└" + "─" * inner + "┘")
    positioned = _position(lines, width, height, "center")
    top_padding = len(positioned) - len(lines)
    styled = _style(positioned[top_padding:], ["normal"] * len(lines), theme, color)
    return positioned[:top_padding] + styled


def timezone_selector(width, height, options, cursor, language, theme="white", color=False):
    box_width = min(60, max(34, width - 4))
    inner = box_width - 2
    lines = [_top_border(box_width, tr(language, "timezone"))]
    for index, value in enumerate(options):
        pointer = "›" if index == cursor else " "
        if value == "system":
            label = tr(language, "system_zone")
        elif index == len(options) - 1:
            label = f"{tr(language, 'custom_zone')}: {value}"
        else:
            label = value
        lines.append("│ " + _pad(f"{pointer} {label}", inner - 2) + " │")
    lines.append("│ " + _pad(_fit(tr(language, "timezone_help"), inner - 2), inner - 2) + " │")
    lines.append("└" + "─" * inner + "┘")
    positioned = _position(lines, width, height, "center")
    top_padding = len(positioned) - len(lines)
    styled = _style(positioned[top_padding:], ["normal"] * len(lines), theme, color)
    return positioned[:top_padding] + styled
