# AIUsage

**简体中文** | [English](README_EN.md)

一个轻量、响应式的 AI CLI 额度终端看板。

在 SSH、VPS 和 Linux 终端中统一查看 Codex、Grok 等 AI CLI 的剩余额度、使用窗口和重置时间。

[介绍](#介绍) · [效果预览](#效果预览) · [一键安装](#一键安装) · [安装完成后](#安装完成后) · [快速使用](#快速使用) · [支持情况](#当前支持情况) · [主题](#主题切换) · [快捷键](#快捷键) · [配置](#配置文件)

## 介绍

AIUsage 直接运行在终端中，不需要 Web 面板或后台 daemon。输入 `aiusage`，即可打开自动适应 SSH、tmux 和 VPS pane 的额度看板。

- Codex、Grok 真实额度读取
- 新用户默认中文，可实时切换 English
- White / Green 前景主题
- 单一总框、居中标题、自然内容尺寸
- 1 / 2 / 3 列响应式布局
- 实时系统时间，额度每 30 秒刷新
- Unicode 进度条与低闪烁局部重绘
- 无 telemetry，不上传额度或配置

## 效果预览

### 真实模式

`aiusage` 只展示具有可靠本地数据源的真实额度。下面是中文界面示意，百分比和时间以你本机 CLI 返回的数据为准：

```text
┌────────────────── AI USAGE ──────────────────┐
│                                              │
│   CODEX                                      │
│   5h     ███████░░░  37% 剩余                │
│   重置：9月03日 02:50 UTC+08                 │
│   Week   ███████░░░  35% 剩余                │
│   重置：9月07日 10:27 UTC+08                 │
│                                              │
│   GROK                                       │
│   Week   █████████░  53% 剩余                │
│   重置：9月05日 23:14 UTC+08                 │
│                                              │
│ 系统时间：2026-09-02 23:43:32 UTC+08         │
│ 数据更新：23:43:15                           │
│                                              │
│ T主题 L语言 P位置 S服务 Z时区 R刷新 Q退出    │
│                                              │
└──────────────────────────────────────────────┘
```

看板只有一个总框，Provider 不会各自套框。框高由内容决定，再把整个内容块放到所选位置，不会强制填满 terminal。

### Demo 模式

`aiusage --demo` 使用固定演示数据，可用于 UI 预览、README 截图、布局测试和中英文测试。界面会明确显示 **`[演示]`**。

Demo/UI 已准备 Claude、Gemini、DeepSeek、Kimi、GLM 和 z.ai，但这些 Provider **尚无真实额度 reader**，演示百分比不能视为实际额度。完整 80×24 文本效果见 [`docs/screenshots/demo-80x24.txt`](docs/screenshots/demo-80x24.txt)。

## 一键安装

```bash
git clone https://github.com/hyd1aa/aiusage.git
cd aiusage
sudo ./install.sh
ai
```

安装脚本可以重复执行，安装 `ai`、`aiusage` 和 AIUsage package，不会覆盖已有用户配置。

## 安装完成后

普通用户只需记住：

```bash
ai
```

它会打开统一管理菜单，可完成启动、Demo、设置、检查更新、环境检查和安全卸载：

```text
╔══════════════════════════════════════╗
║               AIUsage                ║
║          AI CLI 额度终端看板         ║
╚══════════════════════════════════════╝
当前版本：v0.1.x
最新版本：v0.1.x
GitHub: https://github.com/hyd1aa/aiusage
----------------------------------------
1. 启动额度看板
2. Demo 演示模式
3. 设置
4. 检查 / 更新版本
5. 运行环境检查
6. 卸载 AIUsage
0. 退出
```

熟悉用户可继续直接运行：

```bash
aiusage
```

`aiusage` 的行为完全不变，仍直接进入真实额度看板。管理菜单和 Dashboard 共用同一份语言、主题、位置、时区及 Provider 配置；退出由管理菜单启动的 Dashboard 后会返回菜单。

版本检查使用短超时与本地缓存，不会阻塞离线使用。更新只接受 `https://github.com/hyd1aa/aiusage` 的正式 Release，明确确认后才下载、校验版本并安装；需要写入 `/usr/local` 时才调用 `sudo`。用户配置不会被更新删除。

## 快速使用

管理菜单：

```bash
ai
```

真实模式：

```bash
aiusage
```

Demo 模式：

```bash
aiusage --demo
```

其他命令：

```bash
aiusage --version
aiusage --help
aiusage --demo --snapshot --size 80x24
```

## 当前支持情况

| Provider | 真实额度 | Demo / UI | 状态 |
| --- | --- | --- | --- |
| Codex | ✅ | ✅ | 已支持 |
| Grok | ✅ | ✅ | 已支持 |
| Claude | ❌ | ✅ | 仅 UI / Demo |
| Gemini | ❌ | ✅ | 仅 UI / Demo |
| DeepSeek | ❌ | ✅ | 仅 UI / Demo |
| Kimi | ❌ | ✅ | 仅 UI / Demo |
| GLM | ❌ | ✅ | 仅 UI / Demo |
| z.ai | ❌ | ✅ | 仅 UI / Demo |

真实模式绝不会使用 Demo 数据冒充额度。启用但没有可靠 reader 的 Provider，会如实显示“未安装”“不可用”或“不支持”。

## 主题切换

新用户默认使用 **White** 主题。运行中按 `T` 可切换：

```text
White ↔ Green
```

主题只控制前景元素：

- 文字
- 总框边线
- 进度条

AIUsage 永远使用用户自己的 terminal background，不设置白色、绿色、灰色或 RGB 背景，也不会用带背景色的空格填充看板。

## 中英文切换

新用户在没有配置文件时默认使用中文。按 `L` 可实时切换中文与 English，并自动保存。

已经保存 `language = "en"` 的用户升级后仍保持 English，不会被强制切回中文。Provider 品牌名称始终保持原名。

## 快捷键

| 按键 | 功能 |
| --- | --- |
| `T` | 白色 / 绿色主题 |
| `L` | 中文 / English |
| `P` | 切换看板位置 |
| `S` | Provider 管理 |
| `Z` | 选择显示时区 |
| `R` | 立即刷新 |
| `Q` | 退出 |
| `Esc` | 退出 |
| `Ctrl+C` | 退出 |

Provider 管理中可使用方向键或 `J` / `K` 选择，`Space` 启用或禁用，`U` / `D` 排序，`Enter` 保存，`Esc` 取消。

## 响应式布局

- 1～2 个 Provider：紧凑单列
- 3 个 Provider：空间允许时 3×1，否则自动换列
- 4 个 Provider：居中 2×2
- 5～6 个 Provider：居中 3×2
- 更窄的 terminal：自动降为 2 列或 1 列

所有布局都只有一个总框。标题相对实际框宽居中，Provider grid 作为紧凑内容块整体居中；框尺寸由内容自然计算，不会强制占满 terminal。80×24 已验证，更小尺寸会自动响应。

## Provider 管理

按 `S` 打开 Provider 管理，可启用、禁用和调整顺序。Real mode 默认启用 Codex、Grok；Demo mode 默认展示 6 个 Provider。选择分别保存，不会把 Demo 配置混入真实 reader。

## 看板位置

按 `P` 在以下位置循环切换，并保存选择：

```text
左上 / 顶部居中 / 右上 / 正中 / 左下 / 底部居中 / 右下
```

移动的是整个自然尺寸看板，不会把框内内容拉散。

## 时间与时区

默认配置 `timezone = "system"`。AIUsage 每次启动和显示时都会使用操作系统当前时区，不会把旧的 system timezone 缓存在配置里；VPS 改变时区后，重新启动即可自动跟随。

按 `Z` 可以选择“跟随系统”、常用 UTC offset，或以 15 分钟步进调整自定义 offset。支持 `UTC-12` 至 `UTC+14`，包括 `UTC+05:30`、`UTC+05:45` 和 `UTC+09:30`。设置会同时作用于系统时间和所有 Reset 时间。

UI 统一显示无歧义的 numeric UTC offset，例如 `UTC+08`、`UTC-04`，不会显示 `CST`、`EST`、`EDT`、`Asia/Shanghai` 或 `America/New_York`。内部仍使用操作系统的 IANA 时区规则，因此夏令时会按目标时间点正确计算。

AIUsage 从 reset epoch 的绝对时间点进行真正的时区转换，而不是只替换标签。例如原始时间为：

```text
2026-09-03 18:50 UTC
```

使用 `timezone = "UTC"`：

```text
9月03日 18:50 UTC
```

使用 `timezone = "UTC+08"`：

```text
9月04日 02:50 UTC+08
```

日期确实从 3 日跨到 4 日。英文界面对应显示 `Sep 04 02:50 UTC+08`。

如果中国用户的 VPS 仍是 UTC，但希望按北京时间查看，可以设置 `timezone = "UTC+08"`；如果 VPS 本身已是 UTC+08，保留 `timezone = "system"` 即可。

## 配置文件

配置保存在：

```text
~/.config/aiusage/config.toml
```

示例：

```toml
language = "zh"
theme = "white"
position = "center"
timezone = "system"
real_providers = ["codex", "grok"]
demo_providers = ["codex", "grok", "deepseek", "claude", "gemini", "kimi"]
```

这里只保存语言、主题、位置、显示时区、启用的 Provider 和排序，不保存 token、cookie、账号、IP、hostname 或额度快照。旧配置没有 `timezone` 时会自动按 `system` 处理，无需迁移。文件采用仅当前用户可读写权限并原子写入；配置缺失、损坏或不可读时会安全回退。完整示例见 [`config.example.toml`](config.example.toml)。

## 隐私与安全

- 不上传 usage 数据、token 或配置
- 不提供账号，不代替用户登录
- 不读取或分享保存的凭据
- 不启动后台 daemon
- 无 telemetry
- Demo mode 不调用真实 adapter、认证信息或远端 usage API

AIUsage 是非官方社区工具。Codex reader 使用 CLI 的本地 app-server rate-limit 方法；Grok reader 读取本地结构化 billing log 的有限尾部。上游接口可能变化，未来可能需要同步更新。AIUsage 不绕过登录、不共享 token，也不伪造客户端身份。安全报告方式见 [`SECURITY.md`](SECURITY.md)。

## 系统要求

- Linux（已测试）
- Python 3.10 或更高版本
- 支持 ANSI 的终端，推荐 UTF-8
- 查看真实 Codex / Grok 额度时，需要对应 CLI 已安装并由用户自行正规登录

macOS 尚未验证，可能可以运行；Windows 当前不支持。设置 `NO_COLOR` 可关闭彩色前景样式，Unicode 边框和进度条仍保留。

## 卸载

推荐从 `ai` 选择“卸载 AIUsage”，可明确选择保留或删除用户配置，并需要二次确认。

```bash
sudo ./uninstall.sh
```

卸载只删除 AIUsage 程序，默认保留 `~/.config/aiusage/`。如需同时删除偏好，请自行明确删除该目录。

## 开发与贡献

```bash
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e '.[dev]'
python -m compileall -q src tests tools
python -m unittest discover -s tests -v
python tools/check_sensitive.py
```

测试完全离线，不需要登录 Codex 或 Grok。新增真实 Provider adapter 必须有可靠、可验证的数据源，禁止伪造真实 usage 或提交凭据。详见 [`CONTRIBUTING.md`](CONTRIBUTING.md) 和 [`CHANGELOG.md`](CHANGELOG.md)。

## License

本项目采用 [MIT License](LICENSE)。Copyright 使用中性的 “AIUsage contributors”，不代表任何个人身份。
