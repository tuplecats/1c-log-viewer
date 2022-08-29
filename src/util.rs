use std::str::FromStr;
use chrono::{Duration, Local, NaiveDateTime, NaiveTime, Timelike};
use regex::Regex;

pub fn parse_date(value: &str) -> Result<NaiveDateTime, regex::Error> {
    let now = Local::now().naive_local();
    let regex = Regex::new(r#"^now-(\d+)([smhdw])$"#)?;

    match regex.captures(value) {
        Some(captures) if captures.len() == 3 => {
            match (captures.get(1), captures.get(2)) {
                (Some(offset), Some(char)) => {
                    let offset = offset.as_str().parse::<u64>()
                        .map_err(|_| regex::Error::Syntax(String::from("Cannot parse number")))?;

                    match char.as_str() {
                        "s" => {
                            Ok(now - Duration::seconds(offset as i64))
                        }
                        "m" => {
                            Ok(now - Duration::minutes(offset as i64))
                        }
                        "h" => {
                            Ok(now - Duration::hours(offset as i64))
                        }
                        "d" => {
                            Ok(now - Duration::days(offset as i64))
                        }
                        "w" => {
                            Ok(now - Duration::weeks(offset as i64))
                        }
                        _ => unreachable!(),
                    }
                },
                _ => Err(regex::Error::Syntax("Invalid captures".to_string()))
            }
        },
        _ => Err(regex::Error::Syntax("Invalid value".to_string()))
    }
}

pub fn parse_time(hour: NaiveDateTime, time: &str) -> NaiveDateTime {
    let minutes_pos = time
        .as_bytes()
        .iter()
        .position(|char| *char == b':')
        .unwrap();
    let seconds_pos = time
        .as_bytes()
        .iter()
        .position(|char| *char == b'.')
        .unwrap();

    let minutes = match u32::from_str(&time[0..minutes_pos]) {
        Ok(v) => v,
        Err(_) => unreachable!()
    };
    let seconds = u32::from_str(&time[(minutes_pos + 1)..seconds_pos]).unwrap();
    let nanos = &time[(seconds_pos + 1)..];
    let nanos_count = nanos.chars().count();
    let nanos = u32::from_str(nanos).unwrap();

    match nanos_count {
        0..=3 => NaiveDateTime::new(
            hour.date(),
            NaiveTime::from_hms_milli(hour.time().hour(), minutes, seconds, nanos),
        ),
        4..=6 => NaiveDateTime::new(
            hour.date(),
            NaiveTime::from_hms_micro(hour.time().hour(), minutes, seconds, nanos),
        ),
        _ => NaiveDateTime::new(
            hour.date(),
            NaiveTime::from_hms_nano(hour.time().hour(), minutes, seconds, nanos),
        ),
    }
}

pub fn sub_strings(string: &str, sub_len: usize) -> Vec<&str> {
    let mut subs = Vec::with_capacity(string.len() * 2 / sub_len);
    let mut iter = string.chars();
    let mut pos = 0;

    while pos < string.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(sub_len) {
            len += ch.len_utf8();
            if ch == '\n' {
                break;
            }
        }
        subs.push(&string[pos..pos + len]);
        pos += len;
    }
    subs
}