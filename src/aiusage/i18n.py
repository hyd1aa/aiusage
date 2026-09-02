TEXT = {
    "en": {
        "demo": "DEMO", "system": "System", "updated": "Usage updated", "reset": "Reset",
        "providers": "Providers", "unavailable": "Unavailable", "not_installed": "Not installed",
        "not_supported": "Not supported", "stale": "stale", "left": "left",
        "help": "T Theme  L Language  P Position  S Providers  R Refresh  Q Exit",
        "select_help": "↑/↓ Select  Space Toggle  U/D Reorder  Enter Save  Esc Cancel",
    },
    "zh": {
        "demo": "演示", "system": "系统时间", "updated": "数据更新", "reset": "重置",
        "providers": "服务", "unavailable": "不可用", "not_installed": "未安装",
        "not_supported": "不支持", "stale": "数据陈旧", "left": "剩余",
        "help": "T 主题  L 语言  P 位置  S 服务  R 刷新  Q 退出",
        "select_help": "↑/↓ 选择  空格 启用  U/D 排序  Enter 保存  Esc 取消",
    },
}


def tr(language, key):
    return TEXT.get(language, TEXT["en"]).get(key, key)
