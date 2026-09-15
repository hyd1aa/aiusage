use chrono::{DateTime, FixedOffset, Local, Offset, TimeZone, Utc};

pub const MINUTES_MIN: i32 = -720;
pub const MINUTES_MAX: i32 = 840;
pub const PRESETS: &[&str] = &["system", "UTC", "UTC+08", "UTC+09", "UTC-04", "UTC-05"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    System,
    Fixed(i32),
}

/// Unicode decimal digits, matching Python's `\d` / int for timezone input.
/// Decimal blocks are contiguous groups of ten (Unicode 15, including the
/// five mathematical styles). Unlike `is_numeric`, this excludes fractions
/// and superscripts, which the reference also rejects.
pub fn decimal_digit(value: char) -> Option<u32> {
    const ZEROES: &[u32] = &[
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10, 0x104a0, 0x10d30, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450, 0x114d0,
        0x11650, 0x116c0, 0x11730, 0x118e0, 0x11950, 0x11c50, 0x11d50, 0x11da0, 0x11f50, 0x16a60,
        0x16ac0, 0x16b50, 0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e4f0,
        0x1e950, 0x1fbf0,
    ];
    ZEROES
        .iter()
        .find_map(|zero| (value as u32).checked_sub(*zero).filter(|n| *n < 10))
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
        Ok((decimal_digit(a).ok_or("invalid timezone")? * 10
            + decimal_digit(b).ok_or("invalid timezone")?) as i32)
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
