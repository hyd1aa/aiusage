import os
import shutil
import subprocess
import sys
import threading

from . import __version__, config
from .cli import main as dashboard_main
from .diagnostics import collect
from .providers import REGISTRY, discover_all
from .render import _fit, _pad, visible_len
from .timezones import PRESETS, valid_timezone
from .updater import REPOSITORY_URL, cached_latest, check_latest, install_release, is_newer


FG = {"white": "\x1b[37m", "green": "\x1b[32m"}
RESET = "\x1b[0m"


TEXT = {
    "zh": {
        "subtitle": "AI CLI 额度终端看板", "current": "当前版本", "latest": "最新版本",
        "unknown": "未知", "checking_failed": "检查失败", "latest_ok": "✓ 已是最新",
        "prompt": "请输入你的选择：", "back": "按 Enter 返回", "invalid": "无效选择。",
        "main": ["启动额度看板", "Demo 演示模式", "设置", "检查 / 更新版本", "运行环境检查", "卸载 AIUsage"],
    },
    "en": {
        "subtitle": "AI CLI usage terminal dashboard", "current": "Current version", "latest": "Latest version",
        "unknown": "Unknown", "checking_failed": "Check failed", "latest_ok": "✓ Up to date",
        "prompt": "Enter your choice: ", "back": "Press Enter to return", "invalid": "Invalid choice.",
        "main": ["Launch dashboard", "Demo mode", "Settings", "Check / Update", "Diagnostics", "Uninstall AIUsage"],
    },
}

POSITION_ZH = {"top-left": "左上", "top-center": "顶部居中", "top-right": "右上", "center": "居中", "bottom-left": "左下", "bottom-center": "底部居中", "bottom-right": "右下"}
DIAGNOSTIC_ZH = {"AIUsage": "AIUsage", "Python": "Python", "Terminal": "终端", "Config": "配置文件", "Codex": "Codex", "Codex usage": "Codex 额度", "Grok": "Grok", "Grok usage": "Grok 额度", "System timezone": "系统时区", "Display timezone": "显示时区", "GitHub": "GitHub 连接"}
DETAIL_ZH = {"installed": "已安装", "not installed": "未安装", "not_installed": "未安装", "readable": "可读取", "unavailable": "不可用", "available": "正常", "unknown": "未知", "ready": "已就绪", "needs_login": "需要登录", "unsupported": "不支持", "disabled_by_user": "用户已禁用", "timeout": "探测超时", "malformed": "探测结果异常"}


class Manager:
    def __init__(self, cfg=None, input_fn=input, output=None):
        self.cfg = cfg or config.load()
        self.input = input_fn
        self.output = output or sys.stdout
        self.latest = cached_latest()
        self.latest_failed = False
        self.color = self.output.isatty() and "NO_COLOR" not in os.environ and os.environ.get("TERM") != "dumb"

    @property
    def text(self):
        return TEXT[self.cfg.language]

    def write(self, value="", accent=False):
        width = max(10, shutil.get_terminal_size((80, 24)).columns)
        for line in str(value).splitlines() or [""]:
            line = _fit(line, width)
            if self.color and line:
                style = "\x1b[1;92m" if accent else FG.get(self.cfg.theme, FG["white"])
                line = style + line + RESET
            print(line, file=self.output)

    def _background_latest(self):
        def worker():
            try:
                self.latest = check_latest()
            except Exception:
                self.latest_failed = True
        threading.Thread(target=worker, daemon=True).start()

    def main_screen(self):
        unicode = self._unicode()
        left, right, horizontal = ("║", "║", "═") if unicode else ("|", "|", "-")
        self.write(("╔" if unicode else "+") + horizontal * 38 + ("╗" if unicode else "+"))
        self.write(left + self._center("AIUsage", 38) + right)
        subtitle = self.text["subtitle"]
        self.write(left + self._center(subtitle, 38) + right)
        self.write(("╚" if unicode else "+") + horizontal * 38 + ("╝" if unicode else "+"))
        self.write(f"{self.text['current']}：v{__version__}" if self.cfg.language == "zh" else f"{self.text['current']}: v{__version__}")
        latest = f"v{self.latest.version}" if self.latest else self.text["checking_failed" if self.latest_failed else "unknown"]
        marker = "  ← NEW" if self.latest and is_newer(self.latest.version, __version__) else ""
        self.write((f"{self.text['latest']}：{latest}" if self.cfg.language == "zh" else f"{self.text['latest']}: {latest}") + marker, bool(marker))
        if self.latest and self.latest.version == __version__:
            self.write(self.text["latest_ok"])
        self.write(f"GitHub: {REPOSITORY_URL}")
        self.write("-" * 40)
        for index, label in enumerate(self.text["main"], 1):
            self.write(f"{index}. {label}")
        self.write("0. " + ("退出" if self.cfg.language == "zh" else "Exit"))
        self.write("-" * 40)

    def _unicode(self):
        return "UTF" in (getattr(self.output, "encoding", "") or "").upper()

    @staticmethod
    def _center(value, width):
        gap = max(0, width - visible_len(value))
        return " " * (gap // 2) + _pad(value, width - gap // 2)

    def run(self):
        self._background_latest()
        while True:
            self.main_screen()
            try:
                choice = self.input(self.text["prompt"]).strip()
            except (EOFError, KeyboardInterrupt):
                self.write()
                return 0
            if choice == "0": return 0
            if choice == "1": dashboard_main([])
            elif choice == "2": dashboard_main(["--demo"])
            elif choice == "3": self.settings()
            elif choice == "4": self.update_menu()
            elif choice == "5": self.diagnostics()
            elif choice == "6":
                if self.uninstall_menu(): return 0
            else: self.write(self.text["invalid"])

    def settings(self):
        while True:
            lang = "中文" if self.cfg.language == "zh" else "English"
            zone = "跟随系统" if self.cfg.timezone == "system" and self.cfg.language == "zh" else "System" if self.cfg.timezone == "system" else self.cfg.timezone
            position = POSITION_ZH[self.cfg.position] if self.cfg.language == "zh" else self.cfg.position
            providers = ", ".join(REGISTRY[key].name for key in self.cfg.real_providers)
            discovery = "开启" if self.cfg.auto_discover and self.cfg.language == "zh" else "关闭" if self.cfg.language == "zh" else "On" if self.cfg.auto_discover else "Off"
            labels = [f"1. {'语言' if self.cfg.language == 'zh' else 'Language'}: {lang}", f"2. {'主题' if self.cfg.language == 'zh' else 'Theme'}: {self.cfg.theme.title()}", f"3. {'看板位置' if self.cfg.language == 'zh' else 'Position'}: {position}", f"4. {'显示时区' if self.cfg.language == 'zh' else 'Display timezone'}: {zone}", f"5. Provider: {providers}", f"6. {'自动发现 Provider' if self.cfg.language == 'zh' else 'Auto-discover providers'}: {discovery}", f"7. {'恢复默认设置' if self.cfg.language == 'zh' else 'Restore defaults'}", f"0. {'返回' if self.cfg.language == 'zh' else 'Back'}"]
            self.write("AIUsage 设置" if self.cfg.language == "zh" else "AIUsage Settings")
            for line in labels: self.write(line)
            choice = self.input(self.text["prompt"]).strip()
            if choice == "0": return
            if choice == "1": self.cfg.language = "en" if self.cfg.language == "zh" else "zh"
            elif choice == "2": self.cfg.theme = "green" if self.cfg.theme == "white" else "white"
            elif choice == "3":
                index = config.POSITIONS.index(self.cfg.position); self.cfg.position = config.POSITIONS[(index + 1) % len(config.POSITIONS)]
            elif choice == "4": self.timezone_menu()
            elif choice == "5": self.provider_menu()
            elif choice == "6": self.cfg.auto_discover = not self.cfg.auto_discover
            elif choice == "7":
                if self._yes("恢复默认设置？ [y/N]: " if self.cfg.language == "zh" else "Restore defaults? [y/N]: "): self.cfg = config.Config()
            else: continue
            config.save(self.cfg)

    def timezone_menu(self):
        options = list(PRESETS) + ["custom"]
        for index, value in enumerate(options, 1):
            label = "跟随系统" if value == "system" and self.cfg.language == "zh" else "System" if value == "system" else "自定义..." if value == "custom" and self.cfg.language == "zh" else "Custom..." if value == "custom" else value
            self.write(f"{index}. {label}")
        choice = self.input(self.text["prompt"]).strip()
        try: selected = options[int(choice) - 1]
        except (ValueError, IndexError): return
        if selected == "custom":
            selected = self.input("UTC offset (for example UTC+05:30): ").strip()
            if not valid_timezone(selected) or selected == "system":
                self.write(self.text["invalid"]); return
        self.cfg.timezone = selected; config.save(self.cfg)

    def provider_menu(self):
        draft = list(self.cfg.real_providers)
        previous = set(draft)
        discovery = discover_all(REGISTRY)
        while True:
            for index, (key, adapter) in enumerate(REGISTRY.items(), 1):
                mark = "x" if key in draft else " "
                state = discovery[key]
                detail = DETAIL_ZH.get(state.reason, state.reason) if self.cfg.language == "zh" else state.reason.replace("_", " ").title()
                self.write(f"{index}. [{mark}] {adapter.name} — {detail}")
            self.write("命令：数字切换，uN/dN 排序，s 保存，0 取消" if self.cfg.language == "zh" else "Commands: number toggle, uN/dN reorder, s save, 0 cancel")
            choice = self.input("> ").strip().lower()
            if choice == "0": return
            if choice == "s":
                selected = set(draft)
                self.cfg.real_providers = draft
                self.cfg.disabled_providers = list(dict.fromkeys(
                    [key for key in self.cfg.disabled_providers if key not in selected]
                    + [key for key in previous - selected]
                ))
                config.save(self.cfg)
                return
            try:
                direction = choice[0] if choice[0] in "ud" else ""; number = int(choice[1:] if direction else choice); key = list(REGISTRY)[number - 1]
            except (ValueError, IndexError): continue
            if direction and key in draft:
                index = draft.index(key); target = max(0, min(len(draft) - 1, index + (-1 if direction == "u" else 1))); draft[index], draft[target] = draft[target], draft[index]
            elif not direction:
                draft.remove(key) if key in draft else draft.append(key)

    def update_menu(self):
        try: self.latest = check_latest(timeout=3)
        except Exception:
            self.write(self.text["checking_failed"]); self.input(self.text["back"]); return
        self.write(f"v{__version__} -> v{self.latest.version}")
        if not is_newer(self.latest.version, __version__):
            self.write(self.text["latest_ok"]); self.input(self.text["back"]); return
        self.write("用户配置将保留：~/.config/aiusage/" if self.cfg.language == "zh" else "User config will be preserved: ~/.config/aiusage/")
        if not self._yes("确认更新？ [y/N]: " if self.cfg.language == "zh" else "Update now? [y/N]: "): return
        ok, version = install_release(self.latest, __version__)
        self.write(("更新完成：v" if self.cfg.language == "zh" else "Updated: v") + version if ok else self.text["checking_failed"])

    def diagnostics(self):
        try: check_latest(timeout=2); github = True
        except Exception: github = False
        for name, ok, detail in collect(self.cfg, github):
            if self.cfg.language == "zh":
                name, detail = DIAGNOSTIC_ZH.get(name, name), DETAIL_ZH.get(detail, detail)
            self.write(f"{'✓' if ok else '!'} {name}: {detail}")
        self.input(self.text["back"])

    def uninstall_menu(self):
        self.write("1. 卸载，保留用户配置\n2. 卸载，并删除用户配置\n0. 取消" if self.cfg.language == "zh" else "1. Uninstall, keep config\n2. Uninstall and remove config\n0. Cancel")
        choice = self.input(self.text["prompt"]).strip()
        if choice not in ("1", "2") or not self._yes("确认卸载？ [y/N]: " if self.cfg.language == "zh" else "Confirm uninstall? [y/N]: "): return False
        prefix = os.environ.get("AIUSAGE_PREFIX", "/usr/local")
        script = os.path.join(prefix, "lib", "aiusage-uninstall.sh")
        command = [script]
        if os.geteuid() != 0: command.insert(0, "sudo")
        subprocess.run(command, env={**os.environ, "PREFIX": prefix}, check=True)
        if choice == "2": shutil.rmtree(config.config_path().parent, ignore_errors=True)
        return True

    def _yes(self, prompt):
        return self.input(prompt).strip().lower() in ("y", "yes")


def main():
    try:
        return Manager().run()
    except KeyboardInterrupt:
        print()
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
