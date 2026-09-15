use chrono::{DateTime, FixedOffset, Local, Offset, TimeZone, Utc};

pub const MINUTES_MIN: i32 = -720;
pub const MINUTES_MAX: i32 = 840;
pub const PRESETS: &[&str] = &["system", "UTC", "UTC+08", "UTC+09", "UTC-04", "UTC-05"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    System,
    Fixed(i32),
}

pub fn parse(value: &str) -> Result<Zone, &'static str> {
    if value == "system" {
        return Ok(Zone::System);
    }
    if value == "UTC" {
        return Ok(Zone::Fixed(0));
    }
    let chars: Vec<_> = value.chars().collect();
    if !value.starts_with("UTC")
        || ![6, 9].contains(&chars.len())
        || !['+', '-'].contains(&chars[3])
    {
        return Err("invalid timezone");
    }
    let pair = |a: char, b: char| -> Result<i32, &'static str> {
        Ok((a.to_digit(10).ok_or("invalid timezone")? * 10
            + b.to_digit(10).ok_or("invalid timezone")?) as i32)
    };
    let hour = pair(chars[4], chars[5])?;
    let minute = if chars.len() == 9 {
        if chars[6] != ':' {
            return Err("invalid timezone");
        }
        pair(chars[7], chars[8])?
    } else {
        0
    };
    let total = (hour * 60 + minute) * if chars[3] == '-' { -1 } else { 1 };
    if minute >= 60 || !(MINUTES_MIN..=MINUTES_MAX).contains(&total) {
        return Err("timezone offset out of range");
    }
    Ok(Zone::Fixed(total))
}

pub fn offset_setting(minutes: i32) -> Result<String, &'static str> {
    if !(MINUTES_MIN..=MINUTES_MAX).contains(&minutes) {
        return Err("timezone offset out of range");
    }
    Ok(offset_label(minutes))
}

pub fn offset_label(minutes: i32) -> String {
    if minutes == 0 {
        return "UTC".into();
    }
    let value = minutes.abs();
    format!(
        "UTC{}{:02}{}",
        if minutes >= 0 { '+' } else { '-' },
        value / 60,
        if value % 60 == 0 {
            String::new()
        } else {
            format!(":{:02}", value % 60)
        }
    )
}

pub fn from_epoch(epoch: f64, setting: &str) -> Result<DateTime<FixedOffset>, &'static str> {
    if !epoch.is_finite() {
        return Err("invalid timestamp");
    }
    let seconds = epoch.floor();
    let nanos = ((epoch - seconds) * 1e9).round().clamp(0.0, 999_999_999.0) as u32;
    let utc = Utc
        .timestamp_opt(seconds as i64, nanos)
        .single()
        .ok_or("invalid timestamp")?;
    Ok(match parse(setting)? {
        Zone::System => utc.with_timezone(&Local).fixed_offset(),
        Zone::Fixed(minutes) => {
            utc.with_timezone(&FixedOffset::east_opt(minutes * 60).ok_or("invalid timezone")?)
        }
    })
}

pub fn label_for(value: &DateTime<FixedOffset>) -> String {
    offset_label(value.offset().fix().local_minus_utc().div_euclid(60))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges() {
        for s in [
            "system",
            "UTC",
            "UTC-12",
            "UTC+14",
            "UTC+05:45",
            "UTC+09:30",
            "UTC-00:00",
        ] {
            assert!(parse(s).is_ok(), "{s}");
        }
        for s in [
            "UTC+15",
            "UTC-13",
            "UTC+05:60",
            "GMT+08",
            "UTC+8",
            "Asia/Shanghai",
            "UTC+14:01",
            "UTC-12:01",
        ] {
            assert!(parse(s).is_err(), "{s}");
        }
    }
    #[test]
    fn all_minute_offsets_roundtrip() {
        for minutes in MINUTES_MIN..=MINUTES_MAX {
            assert_eq!(
                parse(&offset_setting(minutes).unwrap()),
                Ok(Zone::Fixed(minutes))
            );
        }
    }
    #[test]
    fn crosses_date_not_just_label() {
        let epoch = Utc
            .with_ymd_and_hms(2026, 9, 3, 18, 50, 0)
            .unwrap()
            .timestamp() as f64;
        assert_eq!(
            from_epoch(epoch, "UTC+05:30")
                .unwrap()
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            "2026-09-04 00:20"
        );
        assert_eq!(
            label_for(&from_epoch(epoch, "UTC+05:30").unwrap()),
            "UTC+05:30"
        );
    }
}
