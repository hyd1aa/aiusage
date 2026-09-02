import datetime as dt
import re
import unicodedata

from .i18n import tr
from .models import Availability, ProviderUsage

ANSI = re.compile(r"\x1b\[[0-9;]*m")
MIN_CELL_WIDTH = 24
GAP = 1


def visible_len(value):
    return sum(2 if unicodedata.east_asian_width(char) in ("W", "F") else 1 for char in ANSI.sub("", value))


def _reset(epoch, compact=False):
    if not isinstance(epoch, (int, float)):
        return "?"
    value = dt.datetime.fromtimestamp(epoch).astimezone()
    now = dt.datetime.now().astimezone()
    if value.date() == now.date():
        return value.strftime("%H:%M")
    return value.strftime("%b%d" if compact else "%b %d %H:%M")


def column_count(count, width):
    if count <= 2:
        return 1
    return max(1, min(3, count, (width + GAP) // (MIN_CELL_WIDTH + GAP)))


def _fit(value, width):
    result = []
    used = 0
    for char in value:
        size = 2 if unicodedata.east_asian_width(char) in ("W", "F") else 1
        if used + size > max(0, width):
            break
        result.append(char)
        used += size
    return "".join(result)


def _pad(value, width):
    value = _fit(value, width)
    return value + " " * max(0, width-visible_len(value))


def _card(provider: ProviderUsage, width: int, height: int, language: str):
    inner = width - 2
    name = provider.name.upper()
    suffix = f" ! {tr(language, 'stale')}" if provider.stale else ""
    body = [name + suffix]
    if provider.availability != Availability.AVAILABLE or not provider.windows:
        body.append(tr(language, provider.availability.value))
    else:
        bar_width = max(4, min(10, inner - 13))
        for window in provider.windows[:2]:
            remaining = max(0, min(100, int(window.remaining_percent)))
            fill = round(remaining * bar_width / 100)
            bar = "█" * fill + "░" * (bar_width-fill)
            label = "Weekly" if window.label == "Week" and language == "en" else window.label
            body.append(f"{_fit(label, 6):<6} {bar} {remaining:>3}%")
            body.append(f"{tr(language, 'reset')}: {_reset(window.reset_at, width < 28)}")
    body = body[:height-2]
    body += [""] * (height-2-len(body))
    top = "┌" + "─" * inner + "┐"
    lines = [top]
    for index, line in enumerate(body):
        line = _fit(line, inner-2)
        lines.append("│ " + _pad(line, inner-2) + " │")
    lines.append("└" + "─" * inner + "┘")
    return lines


def _position(lines, terminal_width, terminal_height, position):
    block_width = max((visible_len(line) for line in lines), default=0)
    block_height = len(lines)
    vertical, horizontal = ("center", "center") if position == "center" else position.split("-", 1)
    left = {"left": 0, "center": max(0, (terminal_width-block_width)//2), "right": max(0, terminal_width-block_width)}[horizontal]
    top = {"top": 0, "center": max(0, (terminal_height-block_height)//2), "bottom": max(0, terminal_height-block_height)}[vertical]
    return [""] * top + [_fit(" " * left + line, terminal_width) for line in lines]


def dashboard(width, height, providers, updated, language="en", position="center", demo=False):
    width, height = max(1, width), max(1, height)
    title = f"AI USAGE [{tr(language, 'demo')}]" if demo else "AI USAGE"
    now = dt.datetime.now().astimezone()
    stamp = updated.astimezone().strftime("%H:%M:%S") if updated else "--:--:--"
    header = f"{title}  {tr(language, 'system')} {now:%H:%M:%S}"
    footer = f"{tr(language, 'updated')} {stamp}  {tr(language, 'help')}"
    columns = column_count(len(providers), width)
    grid_width = width if columns > 1 else min(44, width)
    cell_width = (grid_width - GAP*(columns-1)) // columns
    rows = (len(providers)+columns-1)//columns
    available = max(4, height-2-(rows-1)*GAP)
    card_height = max(4, available//max(1, rows))
    cards = [_card(item, cell_width, card_height, language) for item in providers]
    grid = []
    for row in range(rows):
        group = cards[row*columns:(row+1)*columns]
        for line_index in range(card_height):
            grid.append((" "*GAP).join(card[line_index] for card in group))
        if row < rows-1:
            grid.extend([""]*GAP)
    layout_width = min(width, max(grid_width, visible_len(header), visible_len(footer)))
    grid_left = max(0, (layout_width-grid_width)//2)
    block = [
        _fit(header, layout_width),
        *(" "*grid_left + line for line in grid),
        _fit(footer, layout_width),
    ]
    return _position(block[:height], width, height, position)


def selector(width, height, registry, enabled, cursor, language):
    box_width = min(58, max(30, width-4))
    inner = box_width-2
    lines = ["┌" + "─"*inner + "┐"]
    title = tr(language, "providers")
    lines.append("│ " + _pad(title, inner-2) + " │")
    for index, (key, adapter) in enumerate(registry.items()):
        mark = "x" if key in enabled else " "
        pointer = "›" if index == cursor else " "
        order = enabled.index(key)+1 if key in enabled else 0
        value = f"{pointer} [{mark}] {adapter.name}" + (f"  {order}" if order else "")
        lines.append("│ " + _pad(value, inner-2) + " │")
    help_text = _fit(tr(language, "select_help"), inner-2)
    lines.append("│ " + _pad(help_text, inner-2) + " │")
    lines.append("└" + "─"*inner + "┘")
    return _position(lines, width, height, "center")
