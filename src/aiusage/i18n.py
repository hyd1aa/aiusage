TEXT = {
    "en": {
        "demo": "DEMO", "system": "System", "updated": "Usage updated", "reset": "Reset",
        "providers": "Providers", "unavailable": "Unavailable", "not_installed": "Not installed",
        "not_supported": "Not supported", "stale": "stale", "left": "left",
        "timezone": "Timezone", "system_zone": "System", "custom_zone": "Custom",
        "help": "T Theme  L Language  P Position  S Providers  Z Timezone  R Refresh  Q Exit",
        "help_compact": "T Theme L Lang P Pos S Prov Z Zone R Refresh Q Exit",
        "select_help": "↑/↓ Select  Space Toggle  U/D Reorder  Enter Save  Esc Cancel",
        "timezone_help": "↑/↓ Select  ←/→ Adjust custom  Enter Save  Esc Cancel",
        "discovered": "New provider discovered",
        "unsupported": "Usage unsupported", "needs_login": "Needs login", "ready": "Ready",
        "timeout": "Discovery timeout", "malformed": "Discovery error",
    },
    "zh": {
        "demo": "演示", "system": "系统时间", "updated": "数据更新", "reset": "重置",
        "providers": "服务", "unavailable": "不可用", "not_installed": "未安装",
        "not_supported": "不支持", "stale": "数据陈旧", "left": "剩余",
        "timezone": "时区", "system_zone": "跟随系统", "custom_zone": "自定义",
        "help": "T 主题  L 语言  P 位置  S 服务  Z 时区  R 刷新  Q 退出",
        "help_compact": "T主题 L语言 P位置 S服务 Z时区 R刷新 Q退出",
        "select_help": "↑/↓ 选择  空格 启用  U/D 排序  Enter 保存  Esc 取消",
        "timezone_help": "↑/↓ 选择  ←/→ 调整自定义  Enter 保存  Esc 取消",
        "discovered": "已发现新服务",
        "unsupported": "额度不支持", "needs_login": "需要登录", "ready": "已就绪",
        "timeout": "探测超时", "malformed": "探测异常",
    },
}


def tr(language, key):
    return TEXT.get(language, TEXT["en"]).get(key, key)
