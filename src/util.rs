use chrono::{Duration, Local, NaiveDateTime};
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