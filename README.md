# AIUsage

**简体中文** | [English](README_EN.md)

一个轻量、响应式的 AI CLI 额度终端看板。

在 SSH、VPS 和 Linux 终端中统一查看 Codex、Grok 等 AI CLI 的剩余额度、使用窗口和重置时间。

[介绍](#介绍) · [效果图](#效果图) · [一键安装](#一键安装) · [使用方法](#使用方法) · [支持情况](#支持情况) · [快捷键](#快捷键) · [配置](#配置) · [开源许可](#开源许可)

## 介绍

AIUsage 直接运行在终端中，不需要 Web 面板，也不需要后台 daemon。输入 `aiusage`，就能打开一个会自动适应 SSH、tmux 和 VPS pane 尺寸的额度看板。

- Codex、Grok 真实额度读取
- 中文 / English 实时切换
- Provider 自由选择和排序
- 1 / 2 / 3 列响应式布局
- 实时系统时间，额度每 30 秒刷新
- Unicode 进度条、低闪烁局部重绘
- 无 telemetry，不上传额度或配置

## 效果图

### 真实模式

直接运行 `aiusage`。只展示具有可靠本地数据源的真实额度；下面是布局示意，百分比和重置时间以你本机 CLI 返回的数据为准。

```text
AI USAGE  系统时间 12:34:56
              ┌──────────────────────────────────────────┐
              │ CODEX                                    │
              │ 5h     ████████░░  83%                   │
              │ 重置: 14:46                              │
              └──────────────────────────────────────────┘

              ┌──────────────────────────────────────────┐
              │ GROK                                     │
              │ Weekly ███████░░░  72%                   │
              │ 重置: Sep 05                             │
              └──────────────────────────────────────────┘
```

### Demo 模式

`aiusage --demo` 使用固定演示数据，适合预览 6 个 Provider 的 3×2 布局。界面会明确显示 **`[演示]`**，其中 Claude、Gemini、DeepSeek、Kimi 等数值不是实际额度。

```text
AI USAGE [演示]  系统时间 12:34:56
┌────────────────────────┐ ┌────────────────────────┐ ┌────────────────────────┐
│ CODEX                  │ │ GROK                   │ │ DEEPSEEK               │
│ 5h     ████████░░  83% │ │ Cycle  ███████░░░  72% │ │ Daily  ███████░░░  66% │
└────────────────────────┘ └────────────────────────┘ └────────────────────────┘

┌────────────────────────┐ ┌────────────────────────┐ ┌────────────────────────┐
│ CLAUDE                 │ │ GEMINI                 │ │ KIMI                   │
│ 5h     █████░░░░░  48% │ │ Daily  █████████░  91% │ │ Monthl ████░░░░░░  37% │
└────────────────────────┘ └────────────────────────┘ └────────────────────────┘
```

完整的 80×24 纯文本效果见 [`docs/screenshots/demo-80x24.txt`](docs/screenshots/demo-80x24.txt)。后续 PNG 可放在 `docs/screenshots/aiusage-demo.png`；截图必须来自 Demo 模式，且不能包含主机名、IP、shell 提示符或其他 pane。

## 一键安装

### Linux

```bash
git clone https://github.com/hyd1aa/aiusage.git
cd aiusage
sudo ./install.sh
aiusage
```

安装脚本可重复执行，只会安装 AIUsage 到 `/usr/local/bin/aiusage` 和 `/usr/local/lib/aiusage`，不会修改已有用户配置。

卸载程序：

```bash
sudo ./uninstall.sh
```

卸载默认保留 `~/.config/aiusage/`。如需连同偏好一起删除，请自行明确删除该目录。

## 使用方法

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
aiusage --help
aiusage --version
aiusage --demo --snapshot --size 80x24
```

首次使用且没有配置文件时，界面默认为中文。按 `L` 可以随时切换到 English，选择会自动保存；已经保存 `language = "en"` 的用户不会被升级覆盖。

## 支持情况

| Provider | 真实额度 | Demo / UI | 状态 |
| --- | --- | --- | --- |
| Codex | 是 | 是 | 已支持 |
| Grok | 是 | 是 | 已支持 |
| Claude | 否 | 是 | UI 已就绪 |
| Gemini | 否 | 是 | UI 已就绪 |
| DeepSeek | 否 | 是 | UI 已就绪 |
| Kimi | 否 | 是 | UI 已就绪 |
| GLM | 否 | 是 | UI 已就绪 |
| z.ai | 否 | 是 | UI 已就绪 |

真实模式绝不会用 Demo 数据冒充额度。启用但没有可靠 reader 的 Provider，会如实显示“未安装”“不可用”或“不支持”。

## 快捷键

| 按键 | 功能 |
| --- | --- |
| `L` | 中文 / English 切换 |
| `P` | 切换整个看板的位置 |
| `S` | 选择、启用和排序 Provider |
| `R` | 立即刷新额度 |
| `Q` | 退出 |
| `Esc` / `Ctrl+C` | 退出 |

Provider 管理中可使用方向键或 `J` / `K` 选择，`Space` 启用或禁用，`U` / `D` 排序，`Enter` 保存，`Esc` 取消。

## 响应式布局

AIUsage 会根据终端尺寸和卡片最小宽度自动选择布局：

- 6 个 Provider：80×24 下优先 3×2
- 4 个 Provider：2×2
- 3 个 Provider：根据宽度选择 3×1 或换行
- 1～2 个 Provider：单列 boxed 布局

按 `P` 可让整张看板在左上、顶部居中、右上、正中、左下、底部居中、右下之间循环移动。

## 配置

配置保存在：

```text
~/.config/aiusage/config.toml
```

只保存语言、看板位置、启用的 Provider 和排序，不保存 token、cookie、credential 或额度快照。文件使用仅当前用户可读写的权限并原子写入；配置缺失、损坏或不可读时会安全回退，不影响启动。

示例见 [`config.example.toml`](config.example.toml)。设置 `NO_COLOR` 后不使用彩色样式；Unicode 边框和进度条仍作为界面结构保留。

## 数据与隐私

- 不上传 usage 数据、token 或配置
- 不提供账号，不代替用户登录
- 不读取或分享保存的凭据
- 不启动后台 daemon
- 无 telemetry
- Demo 模式不调用真实 adapter，不访问认证信息或远端 usage API

## 兼容性与说明

AIUsage 是非官方社区工具。真实额度读取依赖用户已经安装并通过正规方式登录的 CLI，以及客户端当前公开或产生的本地结构化状态。

Codex reader 使用 CLI 的本地 app-server rate-limit 方法；Grok reader 读取本地结构化 billing log 的有限尾部。上游 CLI 的内部接口可能变化，未来可能需要同步更新。AIUsage 不提供账号、不绕过登录、不共享 token，也不伪造客户端身份。

运行要求：

- Linux（已测试）
- Python 3.10 或更高版本
- 支持 ANSI 的终端，推荐 UTF-8
- 查看真实 Codex / Grok 额度时，需要对应 CLI 已安装并由用户自行登录

macOS 尚未验证，可能可以运行；Windows 当前不支持。

## 常见问题

- **未安装**：请通过 Provider 的正常方式安装官方 CLI；AIUsage 不负责登录。
- **不可用**：按 `R` 刷新，并确认对应 CLI 正常且已产生额度状态。
- **不支持**：Provider 可用于 UI / Demo，但还没有经过验证的真实 reader。
- **边框乱码**：检查 UTF-8 locale 和终端字体的 box-drawing 字符支持。
- **配置损坏**：修复或删除 `~/.config/aiusage/config.toml` 即可恢复默认值。

## 开发

```bash
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e '.[dev]'
python -m compileall -q src tests tools
python -m unittest discover -s tests -v
python tools/check_sensitive.py
```

测试完全离线，不需要登录 Codex 或 Grok。新增真实 Provider adapter 必须有可靠、可验证的数据源，禁止伪造真实 usage 或提交凭据。参与方式见 [`CONTRIBUTING.md`](CONTRIBUTING.md)。

## 开源许可

本项目采用 [MIT License](LICENSE)。Copyright 使用中性的 “AIUsage contributors”，不代表任何个人身份。
