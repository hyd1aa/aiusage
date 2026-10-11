#!/usr/bin/env bash
# ==============================================================================
# AIUsage 一键安装脚本
# 项目地址: https://github.com/hyd1aa/aiusage
# 用法: curl -fsSL https://raw.githubusercontent.com/hyd1aa/aiusage/main/scripts/install.sh | bash
# ==============================================================================

set -euo pipefail

REPO="hyd1aa/aiusage"
DEFAULT_FALLBACK_VERSION="v0.3.1"

echo "========================================================"
echo "           正在安装 AIUsage (终端额度看板)...           "
echo "========================================================"

# 1. 识别操作系统
OS="$(uname -s)"
case "$OS" in
    Linux*)  OS_TYPE="linux" ;;
    Darwin*) OS_TYPE="macos" ;;
    *)
        echo "错误：不支持的操作系统 ($OS)。" >&2
        echo "Error: Unsupported operating system ($OS). AIUsage supports Linux and macOS." >&2
        exit 1
        ;;
esac

# 2. 识别系统架构
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        ARCH_TYPE="amd64"
        ;;
    aarch64|arm64|armv8*)
        ARCH_TYPE="arm64"
        ;;
    *)
        echo "错误：不支持的硬件架构 ($ARCH)。" >&2
        echo "Error: Unsupported architecture ($ARCH). Precompiled binaries support x86_64 and aarch64." >&2
        exit 1
        ;;
esac

PLATFORM="${OS_TYPE}-${ARCH_TYPE}"
echo "✓ 检测到系统环境: $PLATFORM ($OS $ARCH)"

# 3. 检查下载工具 (curl 或 wget)
if command -v curl >/dev/null 2>&1; then
    DOWNLOADER="curl"
elif command -v wget >/dev/null 2>&1; then
    DOWNLOADER="wget"
else
    echo "错误：系统中未找到 curl 或 wget，无法下载安装文件。" >&2
    echo "请先安装 curl 或 wget 后重试：" >&2
    echo "  - Debian / Ubuntu: sudo apt update && sudo apt install -y curl" >&2
    echo "  - CentOS / RHEL:   sudo yum install -y curl" >&2
    echo "  - Alpine Linux:    sudo apk add curl" >&2
    exit 1
fi

download_file() {
    local url="$1"
    local dest="$2"
    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL "$url" -o "$dest"
    else
        wget -qO "$dest" "$url"
    fi
}

fetch_text() {
    local url="$1"
    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL -H "User-Agent: AIUsage-Installer" "$url" 2>/dev/null || true
    else
        wget -qO- --header="User-Agent: AIUsage-Installer" "$url" 2>/dev/null || true
    fi
}

# 4. 获取最新 Release 版本号
VERSION="${AIUSAGE_VERSION:-}"
if [ -z "$VERSION" ]; then
    echo "• 正在获取最新发布版本..."
    # 方式一：检查 GitHub /releases/latest 重定向链接（避免触发 API 速率限制）
    if [ "$DOWNLOADER" = "curl" ]; then
        REDIRECT_URL=$(curl -fsSLI -o /dev/null -w "%{url_effective}" "https://github.com/${REPO}/releases/latest" 2>/dev/null || true)
        TAG_FROM_REDIRECT=$(echo "$REDIRECT_URL" | awk -F/ '{print $NF}')
        if echo "$TAG_FROM_REDIRECT" | grep -q '^v[0-9]'; then
            VERSION="$TAG_FROM_REDIRECT"
        fi
    fi

    # 方式二：调用 GitHub API
    if [ -z "$VERSION" ]; then
        API_RESP=$(fetch_text "https://api.github.com/repos/${REPO}/releases/latest")
        TAG_FROM_API=$(echo "$API_RESP" | grep -o '"tag_name": *"[^"]*"' | head -n1 | cut -d'"' -f4 || true)
        if echo "$TAG_FROM_API" | grep -q '^v[0-9]'; then
            VERSION="$TAG_FROM_API"
        fi
    fi

    # 方式三：兜底稳定版本
    if [ -z "$VERSION" ]; then
        echo "提示：无法自动从 GitHub 获取最新版本号，使用内置稳定版本 $DEFAULT_FALLBACK_VERSION"
        VERSION="$DEFAULT_FALLBACK_VERSION"
    fi
fi

echo "✓ 目标安装版本: $VERSION"

# 5. 准备临时下载目录
TMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t 'aiusage-install.XXXXXX')
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

# 6. 下载预编译文件
TARBALL_URL="https://github.com/${REPO}/releases/download/${VERSION}/aiusage-${VERSION}-${PLATFORM}.tar.gz"
BINARY_URL="https://github.com/${REPO}/releases/download/${VERSION}/aiusage-${VERSION}-${PLATFORM}"
FALLBACK_BINARY_URL="https://github.com/${REPO}/releases/download/${VERSION}/aiusage-${PLATFORM}"

STAGED_BIN="$TMP_DIR/aiusage"
DOWNLOAD_SUCCESS=0

echo "• 正在下载预编译二进制包 ($PLATFORM)..."

# 优先尝试 tar.gz 归档包
if download_file "$TARBALL_URL" "$TMP_DIR/pkg.tar.gz" 2>/dev/null; then
    if tar -xzf "$TMP_DIR/pkg.tar.gz" -C "$TMP_DIR" 2>/dev/null; then
        if [ -f "$TMP_DIR/aiusage" ]; then
            DOWNLOAD_SUCCESS=1
        else
            FOUND_BIN=$(find "$TMP_DIR" -type f -name "aiusage" | head -n1)
            if [ -n "$FOUND_BIN" ]; then
                mv "$FOUND_BIN" "$STAGED_BIN"
                DOWNLOAD_SUCCESS=1
            fi
        fi
    fi
fi

# 若无 tar.gz，尝试下载版本命名的独立二进制文件
if [ "$DOWNLOAD_SUCCESS" -eq 0 ]; then
    if download_file "$BINARY_URL" "$STAGED_BIN" 2>/dev/null; then
        DOWNLOAD_SUCCESS=1
    fi
fi

# 若失败，尝试通用命名的独立二进制文件
if [ "$DOWNLOAD_SUCCESS" -eq 0 ]; then
    if download_file "$FALLBACK_BINARY_URL" "$STAGED_BIN" 2>/dev/null; then
        DOWNLOAD_SUCCESS=1
    fi
fi

# 下载失败降级提示
if [ "$DOWNLOAD_SUCCESS" -eq 0 ] || [ ! -f "$STAGED_BIN" ]; then
    echo "" >&2
    echo "========================================================" >&2
    echo "错误：下载 AIUsage 预编译二进制失败！" >&2
    echo "可能原因：网络超时、GitHub 访问受限或对应版本暂无当前架构构建。" >&2
    echo "" >&2
    echo "【手动安装备选方案】" >&2
    echo "1. 访问 GitHub Releases 页面手动下载：" >&2
    echo "   https://github.com/${REPO}/releases" >&2
    echo "   下载 aiusage-${VERSION}-${PLATFORM} 后执行：" >&2
    echo "   sudo mv aiusage-* /usr/local/bin/aiusage" >&2
    echo "   sudo chmod +x /usr/local/bin/aiusage" >&2
    echo "" >&2
    echo "2. 从源码自行编译（需已安装 Rust 1.90+）：" >&2
    echo "   git clone https://github.com/${REPO}.git" >&2
    echo "   cd aiusage/rust && cargo build --release --bin aiusage" >&2
    echo "   sudo cp target/release/aiusage /usr/local/bin/aiusage" >&2
    echo "========================================================" >&2
    exit 1
fi

chmod +x "$STAGED_BIN"

# 7. 运行自测验证二进制完整性
if ! "$STAGED_BIN" --version >/dev/null 2>&1; then
    echo "错误：下载的二进制文件校验失败或与当前系统 ABI 不兼容。" >&2
    exit 1
fi

# 8. 确定安装目录与权限
PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="${PREFIX}/bin"
LIB_DIR="${PREFIX}/lib"

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
    if [ ! -w "$BIN_DIR" ] 2>/dev/null || { [ ! -d "$BIN_DIR" ] && [ ! -w "$PREFIX" ] 2>/dev/null; }; then
        if command -v sudo >/dev/null 2>&1; then
            SUDO="sudo"
        else
            echo "提示：无 root/sudo 权限，自动降级安装至用户主目录 ~/.local/bin"
            PREFIX="$HOME/.local"
            BIN_DIR="${PREFIX}/bin"
            LIB_DIR="${PREFIX}/lib"
        fi
    fi
fi

$SUDO mkdir -p "$BIN_DIR" "$LIB_DIR"

# 安装主程序
$SUDO cp -f "$STAGED_BIN" "$BIN_DIR/aiusage"
$SUDO chmod 0755 "$BIN_DIR/aiusage"

# 9. 智能配置 'ai' 命令快捷入口
INSTALL_AI_LINK=1
EXISTING_AI=$(command -v ai 2>/dev/null || true)
if [ -n "$EXISTING_AI" ]; then
    if [ -L "$EXISTING_AI" ]; then
        TARGET="$(readlink "$EXISTING_AI" 2>/dev/null || true)"
        case "$TARGET" in
            *aiusage*) ;; # 已指向 aiusage，可以安全更新
            *) INSTALL_AI_LINK=0 ;;
        esac
    else
        INSTALL_AI_LINK=0
    fi
fi

if [ -e "$BIN_DIR/ai" ] && [ ! -L "$BIN_DIR/ai" ]; then
    INSTALL_AI_LINK=0
fi

if [ "$INSTALL_AI_LINK" -eq 1 ]; then
    $SUDO ln -sf aiusage "$BIN_DIR/ai"
fi

# 10. 生成卸载脚本与元数据标记，确保菜单内的“卸载”功能正常可用
UNINSTALL_SCRIPT="$LIB_DIR/aiusage-uninstall.sh"
TMP_UNINSTALLER="$TMP_DIR/aiusage-uninstall.sh"
cat <<'EOF' > "$TMP_UNINSTALLER"
#!/bin/sh
set -eu
PREFIX=${PREFIX:-/usr/local}
for name in aiusage ai; do
    target="$PREFIX/bin/$name"
    if [ -L "$target" ]; then
        link_target="$(readlink "$target" 2>/dev/null || true)"
        case "$link_target" in
            *aiusage*) rm -f -- "$target" ;;
        esac
    elif [ -f "$target" ] && [ "$name" = "aiusage" ]; then
        rm -f -- "$target"
    fi
done
rm -rf -- "$PREFIX/lib/aiusage" "$PREFIX/lib/aiusage-uninstall.sh"
echo "AIUsage program files removed."
echo "User config preserved: ~/.config/aiusage/"
EOF

$SUDO mkdir -p "$LIB_DIR/aiusage"
$SUDO touch "$LIB_DIR/aiusage/.aiusage-rust-owned"
$SUDO cp -f "$TMP_UNINSTALLER" "$UNINSTALL_SCRIPT"
$SUDO chmod 0755 "$UNINSTALL_SCRIPT"

# 11. 检查 PATH 环境变量
case ":$PATH:" in
    *:"$BIN_DIR":*) ;;
    *)
        echo ""
        echo "注意：安装目录 $BIN_DIR 暂未包含在当前 PATH 环境变量中。"
        echo "建议在终端配置文件（~/.bashrc 或 ~/.zshrc）中添加："
        echo "  export PATH=\"$BIN_DIR:\$PATH\""
        echo "并执行 source ~/.bashrc 使其生效。"
        ;;
esac

# 12. 安装完成提示
VER_STR=$("$BIN_DIR/aiusage" --version 2>/dev/null || echo "AIUsage $VERSION")

echo ""
echo "========================================================"
echo "  ✓ $VER_STR 安装成功！开箱即用"
echo "========================================================"
if [ "$INSTALL_AI_LINK" -eq 1 ]; then
    echo "  • 统一管理菜单 : ai"
else
    echo "  • 统一管理菜单 : aiusage --menu  (第三方 'ai' 命令已保留)"
fi
echo "  • 实时额度看板 : aiusage"
echo "  • 演示预览模式 : aiusage --demo"
echo "========================================================"
echo "直接在终端中运行 'ai' 或 'aiusage' 即可启动！"
