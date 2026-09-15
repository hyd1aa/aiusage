use crate::{
    config::{self, Config, POSITIONS},
    diagnostics,
    providers::{self, DiscoveryResult},
    render,
    updater::{self, ReleaseInfo},
    PROVIDERS,
};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

type Discovery = std::collections::HashMap<String, DiscoveryResult>;
type LatestResult = Result<ReleaseInfo, String>;
pub trait Actions {
    fn latest(&mut self, timeout: Duration) -> LatestResult;
    fn background_latest(&mut self) -> Option<Receiver<LatestResult>> {
        None
    }
    fn discover(&mut self) -> Discovery;
    fn diagnostics(&mut self, cfg: &Config, github: bool) -> Vec<diagnostics::Row>;
    fn launch(&mut self, demo: bool);
    fn install(&mut self, info: &ReleaseInfo) -> Result<String, String>;
    fn uninstall(&mut self, remove_config: bool) -> Result<(), String>;
}
pub struct Production;
impl Actions for Production {
    fn latest(&mut self, timeout: Duration) -> LatestResult {
        updater::check_latest(timeout).map_err(String::from)
    }
    fn background_latest(&mut self) -> Option<Receiver<LatestResult>> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(updater::check_latest(Duration::from_secs(2)).map_err(String::from));
        });
        Some(rx)
    }
    fn discover(&mut self) -> Discovery {
        providers::discover_all()
    }
    fn diagnostics(&mut self, cfg: &Config, github: bool) -> Vec<diagnostics::Row> {
        diagnostics::collect(cfg, Some(github))
    }
    fn launch(&mut self, demo: bool) {
        crate::cli::main(if demo { vec!["--demo".into()] } else { vec![] });
    }
    fn install(&mut self, info: &ReleaseInfo) -> Result<String, String> {
        // Preserve the reference updater's default installation prefix.
        updater::install_release(info, crate::VERSION, std::path::Path::new("/usr/local"))
            .map_err(String::from)
    }
    fn uninstall(&mut self, remove_config: bool) -> Result<(), String> {
        let prefix = std::env::var_os("AIUSAGE_PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/local"));
        let script = prefix.join("lib/aiusage-uninstall.sh");
        let mut command = if unsafe { libc::geteuid() } != 0 {
            let mut cmd = Command::new("sudo");
            cmd.arg(&script);
            cmd
        } else {
            Command::new(&script)
        };
        if !command
            .env("PREFIX", &prefix)
            .status()
            .map_err(|_| "uninstall failed")?
            .success()
        {
            return Err("uninstall failed".into());
        }
        if remove_config {
            let path = config::config_path();
            let parent = path.parent().ok_or("invalid config path")?;
            // Config removal is narrowly scoped to the app directory, after consent.
            if parent.file_name() != Some(std::ffi::OsStr::new("aiusage")) {
                return Err("invalid config path".into());
            }
            if parent.exists() {
                std::fs::remove_dir_all(parent).map_err(|_| "config removal failed")?;
            }
        }
        Ok(())
    }
}
pub struct Manager<I: BufRead, O: Write, A: Actions> {
    pub cfg: Config,
    pub input: I,
    pub output: O,
    pub actions: A,
    pub latest: Option<ReleaseInfo>,
    pub latest_failed: bool,
    pub color: bool,
    pub unicode: bool,
    pub width: usize,
    pub live_width: Option<fn() -> usize>,
    pub config_path: PathBuf,
}
impl<I: BufRead, O: Write, A: Actions> Manager<I, O, A> {
    pub fn new(cfg: Config, input: I, output: O, actions: A) -> Self {
        Self {
            cfg,
            input,
            output,
            actions,
            latest: None,
            latest_failed: false,
            color: false,
            unicode: true,
            width: 80,
            live_width: None,
            config_path: config::config_path(),
        }
    }
    fn text<'a>(&self, zh: &'a str, en: &'a str) -> &'a str {
        if self.cfg.language == "zh" {
            zh
        } else {
            en
        }
    }
    pub fn write(&mut self, value: &str, accent: bool) -> io::Result<()> {
        if let Some(width) = self.live_width {
            self.width = width();
        }
        let rows: Vec<_> = if value.is_empty() {
            vec![""]
        } else {
            value.lines().collect()
        };
        for line in rows {
            let line = render::fit(line, self.width.max(10));
            if self.color && !line.is_empty() {
                let style = if accent {
                    "1;92"
                } else if self.cfg.theme == "green" {
                    "32"
                } else {
                    "37"
                };
                writeln!(self.output, "\x1b[{style}m{line}\x1b[0m")?;
            } else {
                writeln!(self.output, "{line}")?;
            }
        }
        Ok(())
    }
    fn ask(&mut self, prompt: &str) -> io::Result<String> {
        write!(self.output, "{prompt}")?;
        self.output.flush()?;
        let mut line = String::new();
        if self.input.read_line(&mut line)? == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        Ok(line.trim().into())
    }
    fn choice(&mut self) -> io::Result<String> {
        self.ask(self.text("请输入你的选择：", "Enter your choice: "))
    }
    fn yes(&mut self, zh: &str, en: &str) -> io::Result<bool> {
        Ok(matches!(
            self.ask(self.text(zh, en))?.to_lowercase().as_str(),
            "y" | "yes"
        ))
    }
    fn back(&mut self) -> io::Result<()> {
        self.ask(self.text("按 Enter 返回", "Press Enter to return"))?;
        Ok(())
    }
    fn save(&self) {
        config::save(&self.cfg, &self.config_path);
    }
    fn invalid(&mut self) -> io::Result<()> {
        self.write(self.text("无效选择。", "Invalid choice."), false)
    }
    fn center(text: &str, width: usize) -> String {
        let gap = width.saturating_sub(render::visible_len(text)) / 2;
        format!("{}{}", " ".repeat(gap), render::pad(text, width - gap))
    }
    pub fn main_screen(&mut self) -> io::Result<()> {
        let (tl, tr, bl, br, h, v) = if self.unicode {
            ("╔", "╗", "╚", "╝", "═", "║")
        } else {
            ("+", "+", "+", "+", "-", "|")
        };
        self.write(&format!("{tl}{}{tr}", h.repeat(38)), false)?;
        self.write(&format!("{v}{}{v}", Self::center("AIUsage", 38)), false)?;
        self.write(
            &format!(
                "{v}{}{v}",
                Self::center(
                    self.text("AI CLI 额度终端看板", "AI CLI usage terminal dashboard"),
                    38
                )
            ),
            false,
        )?;
        self.write(&format!("{bl}{}{br}", h.repeat(38)), false)?;
        self.write(
            &render::label_value(
                self.text("当前版本", "Current version"),
                &format!("v{}", crate::VERSION),
                &self.cfg.language,
            ),
            false,
        )?;
        let latest = self
            .latest
            .as_ref()
            .map(|info| format!("v{}", info.version))
            .unwrap_or_else(|| {
                if self.latest_failed {
                    self.text("检查失败", "Check failed")
                } else {
                    self.text("未知", "Unknown")
                }
                .into()
            });
        let newer = self
            .latest
            .as_ref()
            .is_some_and(|info| updater::is_newer(&info.version, crate::VERSION));
        self.write(
            &format!(
                "{}{}",
                render::label_value(
                    self.text("最新版本", "Latest version"),
                    &latest,
                    &self.cfg.language
                ),
                if newer { "  ← NEW" } else { "" }
            ),
            newer,
        )?;
        if self
            .latest
            .as_ref()
            .is_some_and(|info| info.version == crate::VERSION)
        {
            self.write(self.text("✓ 已是最新", "✓ Up to date"), false)?;
        }
        self.write(&format!("GitHub: {}", updater::REPOSITORY_URL), false)?;
        self.write(&"-".repeat(40), false)?;
        for (index, (zh, en)) in [
            ("启动额度看板", "Launch dashboard"),
            ("Demo 演示模式", "Demo mode"),
            ("设置", "Settings"),
            ("检查 / 更新版本", "Check / Update"),
            ("运行环境检查", "Diagnostics"),
            ("卸载 AIUsage", "Uninstall AIUsage"),
        ]
        .iter()
        .enumerate()
        {
            self.write(&format!("{}. {}", index + 1, self.text(zh, en)), false)?;
        }
        self.write(&format!("0. {}", self.text("退出", "Exit")), false)?;
        self.write(&"-".repeat(40), false)
    }
    pub fn run(&mut self) -> io::Result<()> {
        let latest = self.actions.background_latest();
        loop {
            if let Some(rx) = &latest {
                if let Ok(value) = rx.try_recv() {
                    match value {
                        Ok(info) => self.latest = Some(info),
                        Err(_) => self.latest_failed = true,
                    }
                }
            }
            self.main_screen()?;
            match self.choice()?.as_str() {
                "0" => return Ok(()),
                "1" => self.actions.launch(false),
                "2" => self.actions.launch(true),
                "3" => self.settings()?,
                "4" => self.update_menu()?,
                "5" => self.diagnostics()?,
                "6" => {
                    if self.uninstall_menu()? {
                        return Ok(());
                    }
                }
                _ => self.invalid()?,
            }
        }
    }
    pub fn settings(&mut self) -> io::Result<()> {
        loop {
            let lang = if self.cfg.language == "zh" {
                "中文"
            } else {
                "English"
            };
            let zone = if self.cfg.timezone == "system" {
                self.text("跟随系统", "System")
            } else {
                &self.cfg.timezone
            };
            let index = POSITIONS
                .iter()
                .position(|p| *p == self.cfg.position)
                .unwrap();
            let position = if self.cfg.language == "zh" {
                [
                    "左上",
                    "顶部居中",
                    "右上",
                    "居中",
                    "左下",
                    "底部居中",
                    "右下",
                ][index]
            } else {
                &self.cfg.position
            };
            let providers = self
                .cfg
                .real_providers
                .iter()
                .filter_map(|key| {
                    PROVIDERS
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, name)| *name)
                })
                .collect::<Vec<_>>()
                .join(", ");
            let discovery = if self.cfg.auto_discover {
                self.text("开启", "On")
            } else {
                self.text("关闭", "Off")
            };
            let rows = vec![
                format!("1. {}: {lang}", self.text("语言", "Language")),
                format!(
                    "2. {}: {}",
                    self.text("主题", "Theme"),
                    if self.cfg.theme == "green" {
                        "Green"
                    } else {
                        "White"
                    }
                ),
                format!("3. {}: {position}", self.text("看板位置", "Position")),
                format!("4. {}: {zone}", self.text("显示时区", "Display timezone")),
                format!("5. Provider: {providers}"),
                format!(
                    "6. {}: {discovery}",
                    self.text("自动发现 Provider", "Auto-discover providers")
                ),
                format!("7. {}", self.text("恢复默认设置", "Restore defaults")),
                format!("0. {}", self.text("返回", "Back")),
            ];
            self.write(self.text("AIUsage 设置", "AIUsage Settings"), false)?;
            for row in rows {
                self.write(&row, false)?;
            }
            match self.choice()?.as_str() {
                "0" => return Ok(()),
                "1" => {
                    self.cfg.language = if self.cfg.language == "zh" {
                        "en"
                    } else {
                        "zh"
                    }
                    .into()
                }
                "2" => {
                    self.cfg.theme = if self.cfg.theme == "white" {
                        "green"
                    } else {
                        "white"
                    }
                    .into()
                }
                "3" => self.cfg.position = POSITIONS[(index + 1) % POSITIONS.len()].into(),
                "4" => self.timezone_menu()?,
                "5" => self.provider_menu()?,
                "6" => self.cfg.auto_discover = !self.cfg.auto_discover,
                "7" => {
                    if self.yes("恢复默认设置？ [y/N]: ", "Restore defaults? [y/N]: ")? {
                        self.cfg = Config::default();
                    }
                }
                _ => continue,
            }
            self.save();
        }
    }
    pub fn timezone_menu(&mut self) -> io::Result<()> {
        let options: Vec<_> = crate::timezones::PRESETS
            .iter()
            .copied()
            .chain(["custom"])
            .collect();
        for (index, value) in options.iter().enumerate() {
            let label = match *value {
                "system" => self.text("跟随系统", "System"),
                "custom" => self.text("自定义...", "Custom..."),
                _ => value,
            };
            self.write(&format!("{}. {label}", index + 1), false)?;
        }
        let Some(index) = python_index(&self.choice()?, options.len()) else {
            return Ok(());
        };
        let selected = if options[index] == "custom" {
            let value = self.ask("UTC offset (for example UTC+05:30): ")?;
            if crate::timezones::parse(&value).is_err() || value == "system" {
                self.invalid()?;
                return Ok(());
            }
            value
        } else {
            options[index].into()
        };
        self.cfg.timezone = selected;
        self.save();
        Ok(())
    }
    pub fn provider_menu(&mut self) -> io::Result<()> {
        let mut draft = self.cfg.real_providers.clone();
        let previous = draft.clone();
        let discovery = self.actions.discover();
        loop {
            for (index, (key, name)) in PROVIDERS.iter().enumerate() {
                let reason = discovery
                    .get(*key)
                    .map(|s| s.reason.as_str())
                    .unwrap_or("unavailable");
                let detail = detail_label(&self.cfg.language, reason);
                self.write(
                    &format!(
                        "{}. [{}] {name} — {detail}",
                        index + 1,
                        if draft.iter().any(|k| k == key) {
                            "x"
                        } else {
                            " "
                        }
                    ),
                    false,
                )?;
            }
            self.write(
                self.text(
                    "命令：数字切换，uN/dN 排序，s 保存，0 取消",
                    "Commands: number toggle, uN/dN reorder, s save, 0 cancel",
                ),
                false,
            )?;
            let choice = self.ask("> ")?.to_lowercase();
            if choice == "0" {
                return Ok(());
            }
            if choice == "s" {
                self.cfg.disabled_providers.retain(|k| !draft.contains(k));
                for key in &previous {
                    if !draft.contains(key) && !self.cfg.disabled_providers.contains(key) {
                        self.cfg.disabled_providers.push(key.clone());
                    }
                }
                self.cfg.real_providers = draft;
                self.save();
                return Ok(());
            }
            let direction = choice.starts_with('u') || choice.starts_with('d');
            let Some(index) = python_index(
                if direction { &choice[1..] } else { &choice },
                PROVIDERS.len(),
            ) else {
                continue;
            };
            let key = PROVIDERS[index].0;
            if let Some(index) = draft.iter().position(|k| k == key) {
                if direction {
                    let target = if choice.starts_with('u') {
                        index.saturating_sub(1)
                    } else {
                        (index + 1).min(draft.len() - 1)
                    };
                    draft.swap(index, target);
                } else {
                    draft.remove(index);
                }
            } else if !direction {
                draft.push(key.into());
            }
        }
    }
    pub fn update_menu(&mut self) -> io::Result<()> {
        let info = match self.actions.latest(Duration::from_secs(3)) {
            Ok(info) => info,
            Err(_) => {
                self.write(self.text("检查失败", "Check failed"), false)?;
                return self.back();
            }
        };
        self.latest = Some(info.clone());
        self.write(&format!("v{} -> v{}", crate::VERSION, info.version), false)?;
        if !updater::is_newer(&info.version, crate::VERSION) {
            self.write(self.text("✓ 已是最新", "✓ Up to date"), false)?;
            return self.back();
        }
        self.write(
            self.text(
                "用户配置将保留：~/.config/aiusage/",
                "User config will be preserved: ~/.config/aiusage/",
            ),
            false,
        )?;
        if !self.yes("确认更新？ [y/N]: ", "Update now? [y/N]: ")? {
            return Ok(());
        }
        match self.actions.install(&info) {
            Ok(version) => self.write(
                &format!("{}{version}", self.text("更新完成：v", "Updated: v")),
                false,
            ),
            Err(_) => self.write(self.text("检查失败", "Check failed"), false),
        }
    }
    pub fn diagnostics(&mut self) -> io::Result<()> {
        let github = self.actions.latest(Duration::from_secs(2)).is_ok();
        for (name, ok, detail) in self.actions.diagnostics(&self.cfg, github) {
            let (name, detail) = if self.cfg.language == "zh" {
                (
                    diagnostic_label(&name).to_string(),
                    detail_label("zh", &detail),
                )
            } else {
                (name, detail)
            };
            self.write(
                &format!("{} {name}: {detail}", if ok { "✓" } else { "!" }),
                false,
            )?;
        }
        self.back()
    }
    pub fn uninstall_menu(&mut self) -> io::Result<bool> {
        self.write(
            self.text(
                "1. 卸载，保留用户配置\n2. 卸载，并删除用户配置\n0. 取消",
                "1. Uninstall, keep config\n2. Uninstall and remove config\n0. Cancel",
            ),
            false,
        )?;
        let choice = self.choice()?;
        if !matches!(choice.as_str(), "1" | "2")
            || !self.yes("确认卸载？ [y/N]: ", "Confirm uninstall? [y/N]: ")?
        {
            return Ok(false);
        }
        if self.actions.uninstall(choice == "2").is_err() {
            self.write(self.text("卸载失败", "Uninstall failed"), false)?;
            return Ok(false);
        }
        Ok(true)
    }
}
fn python_index(value: &str, len: usize) -> Option<usize> {
    let index = crate::cli::python_integer(value)?.checked_sub(1)?;
    let index = if index < 0 { len as i64 + index } else { index };
    if index < 0 || index >= len as i64 {
        None
    } else {
        Some(index as usize)
    }
}
fn detail_label(language: &str, detail: &str) -> String {
    if language != "zh" {
        return detail
            .replace('_', " ")
            .split(' ')
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    match detail {
        "installed" => "已安装",
        "not installed" | "not_installed" => "未安装",
        "readable" => "可读取",
        "unavailable" => "不可用",
        "available" => "正常",
        "unknown" => "未知",
        "ready" => "已就绪",
        "needs_login" => "需要登录",
        "unsupported" => "不支持",
        "disabled_by_user" => "用户已禁用",
        "timeout" => "探测超时",
        "malformed" => "探测结果异常",
        _ => detail,
    }
    .into()
}
fn diagnostic_label(name: &str) -> &str {
    match name {
        "Terminal" => "终端",
        "Config" => "配置文件",
        "Codex usage" => "Codex 额度",
        "Grok usage" => "Grok 额度",
        "System timezone" => "系统时区",
        "Display timezone" => "显示时区",
        "GitHub" => "GitHub 连接",
        _ => name,
    }
}

pub fn main() -> i32 {
    use std::io::IsTerminal;
    let _signals = MenuSignals::enter();
    let mut manager = Manager::new(
        config::load(&config::config_path()),
        io::BufReader::with_capacity(1, RawInput),
        io::stdout(),
        Production,
    );
    manager.latest = updater::cached_latest();
    manager.width = crate::cli::terminal_size().0;
    manager.live_width = Some(|| crate::cli::terminal_size().0);
    manager.color = io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").unwrap_or_default() != "dumb";
    let result = manager.run();
    match result {
        Ok(()) => 0,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::UnexpectedEof | io::ErrorKind::Interrupted
            ) =>
        {
            println!();
            0
        }
        Err(_) => {
            eprintln!("AIUsage manager I/O unavailable");
            1
        }
    }
}
struct RawInput;
impl io::Read for RawInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if MENU_INTERRUPTED.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            let mut fd = libc::pollfd {
                fd: 0,
                events: libc::POLLIN,
                revents: 0,
            };
            if unsafe { libc::poll(&mut fd, 1, 100) } > 0 {
                break;
            }
        }
        let count = unsafe { libc::read(0, buffer.as_mut_ptr().cast(), buffer.len()) };
        if count < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(count as usize)
        }
    }
}
static MENU_INTERRUPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
extern "C" fn interrupt_menu(_: libc::c_int) {
    MENU_INTERRUPTED.store(true, std::sync::atomic::Ordering::Relaxed);
}
struct MenuSignals(Vec<(libc::c_int, libc::sighandler_t)>);
impl MenuSignals {
    fn enter() -> Self {
        MENU_INTERRUPTED.store(false, std::sync::atomic::Ordering::Relaxed);
        Self(
            [libc::SIGINT, libc::SIGTERM]
                .into_iter()
                .map(|signal| {
                    let old = unsafe { libc::signal(signal, interrupt_menu as libc::sighandler_t) };
                    (signal, old)
                })
                .collect(),
        )
    }
}
impl Drop for MenuSignals {
    fn drop(&mut self) {
        for &(signal, old) in &self.0 {
            unsafe {
                libc::signal(signal, old);
            }
        }
    }
}
