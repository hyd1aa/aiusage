pub fn tr<'a>(language: &str, key: &'a str) -> &'a str {
    let pair = match key {
        "demo" => ("DEMO", "演示"),
        "system" => ("System", "系统时间"),
        "updated" => ("Usage updated", "数据更新"),
        "reset" => ("Reset", "重置"),
        "providers" => ("Providers", "服务"),
        "unavailable" => ("Unavailable", "不可用"),
        "not_installed" => ("Not installed", "未安装"),
        "not_supported" => ("Not supported", "不支持"),
        "stale" => ("stale", "数据陈旧"),
        "left" => ("left", "剩余"),
        "timezone" => ("Timezone", "时区"),
        "system_zone" => ("System", "跟随系统"),
        "custom_zone" => ("Custom", "自定义"),
        "help" => (
            "T Theme  L Language  P Position  S Providers  Z Timezone  R Refresh  Q Exit",
            "T 主题  L 语言  P 位置  S 服务  Z 时区  R 刷新  Q 退出",
        ),
        "help_compact" => (
            "T Theme L Lang P Pos S Prov Z Zone R Refresh Q Exit",
            "T主题 L语言 P位置 S服务 Z时区 R刷新 Q退出",
        ),
        "select_help" => (
            "↑/↓ Select  Space Toggle  U/D Reorder  Enter Save  Esc Cancel",
            "↑/↓ 选择  空格 启用  U/D 排序  Enter 保存  Esc 取消",
        ),
        "timezone_help" => (
            "↑/↓ Select  ←/→ Adjust custom  Enter Save  Esc Cancel",
            "↑/↓ 选择  ←/→ 调整自定义  Enter 保存  Esc 取消",
        ),
        "discovered" => ("New provider discovered", "已发现新服务"),
        "unsupported" => ("Usage unsupported", "额度不支持"),
        "needs_login" => ("Needs login", "需要登录"),
        "ready" => ("Ready", "已就绪"),
        "timeout" => ("Discovery timeout", "探测超时"),
        "malformed" => ("Discovery error", "探测异常"),
        _ => return key,
    };
    if language == "zh" {
        pair.1
    } else {
        pair.0
    }
}
