use std::{
    fs::File,
    io,
    io::Read,
    ops::Index,
    str::FromStr,
    sync::mpsc::{channel, Receiver, Sender},
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use indexmap::IndexMap;
use walkdir::WalkDir;
use crate::util::read_until;

mod compiler;
mod value;
pub mod logdata;

pub use value::*;
pub use compiler::{Compiler, Query};

#[derive(Debug, Clone)]
pub struct LogString {
    pub time: Value,
    pub duration: Value,
    pub event: Value,
    pub process: Value,
    pub thread: Value,
    pub fields: IndexMap<String, Value>,
}

impl Default for LogString {
    fn default() -> Self {
        LogString {
            time: Value::String(String::new()),
            duration: Value::String(String::new()),
            event: Value::String(String::new()),
            process: Value::String(String::new()),
            thread: Value::String(String::new()),
            fields: IndexMap::new(),
        }
    }
}

unsafe impl Send for LogString {}
unsafe impl Sync for LogString {}

impl LogString {
    pub fn set_value(&mut self, name: &str, value: Value) {
        match name {
            "process" => self.process = value,
            "OSThread" => self.thread = value,
            _ => { self.fields.insert(name.to_string(), value); },
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        match name {
            "time" => Some(self.time.clone()),
            "duration" => Some(self.duration.clone()),
            "event" => Some(self.event.clone()),
            "process" => Some(self.process.clone()),
            "thread" => Some(self.thread.clone()),
            _ => self.fields.get(name).map(|s| s.clone()),
        }
    }
}

enum ParseState {
    StartLogLine,
    EventField,
    Duration,
    Key,
    Value,
    Finish,
}

#[derive(PartialEq)]
enum ParseValueState {
    BeginParse,
    ReadValueUntil(u8),
    ReadValueToNext,
    Finish((usize, u8)),
}

pub struct LogParser;

impl LogParser {
    pub fn parse(dir: String) -> Receiver<LogString> {
        let (sender, receiver) = channel();
        std::thread::spawn(move || LogParser::parse_dir(dir, sender));
        receiver
    }

    // А может сделать итератор, который парсит
    fn parse_dir(path: String, sender: Sender<LogString>) -> io::Result<()> {
        let walk = WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                !e.file_type().is_dir() && e.file_name().to_string_lossy().ends_with(".log")
            });

        let regex = regex::Regex::new(r#"^\d{8}[.]log$"#).unwrap();
        let mut files = walk
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if regex.is_match(&name) {
                    let year = 2000 + name[0..2].parse::<i32>().unwrap();
                    let month = name[2..4].parse::<u32>().unwrap();
                    let day = name[4..6].parse::<u32>().unwrap();
                    let hour = name[6..8].parse::<u32>().unwrap();
                    Some((e, NaiveDate::from_ymd(year, month, day).and_hms(hour, 0, 0)))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        files.sort_by(|(_, name), (_, name2)| {
            name.cmp(name2)
        });

        for (entry, _) in files {
            if regex::Regex::new(r#"^\d{8}[.]log$"#)
                .unwrap()
                .is_match(entry.file_name().to_string_lossy().as_ref())
            {
                Self::parse_file(entry.path().to_string_lossy().as_ref(), sender.clone())?
            }
        }

        Ok(())
    }

    fn parse_date(file_name: &str) -> chrono::NaiveDateTime {
        let time_str = regex::Regex::new(r#"(\d{8})[.]log"#)
            .unwrap()
            .captures(file_name)
            .map(|capture| capture.index(0).to_string())
            .unwrap();

        let year = 2000 + i32::from_str(&time_str[0..2]).unwrap();
        let month = u32::from_str(&time_str[2..4]).unwrap();
        let day = u32::from_str(&time_str[4..6]).unwrap();
        let hour = u32::from_str(&time_str[6..8]).unwrap();

        chrono::NaiveDateTime::new(
            NaiveDate::from_ymd(year, month, day),
            NaiveTime::from_hms(hour, 0, 0),
        )
    }

    fn parse_file(path: &str, sender: Sender<LogString>) -> std::io::Result<()> {
        let mut data = String::new();
        File::open(path)?.read_to_string(&mut data)?;

        // Удалим BOM
        data.remove(0);

        let date = Self::parse_date(path);

        let data_str = data.as_str();
        let mut state = ParseState::StartLogLine;
        let mut iter = data.as_bytes().iter().map(|b| *b).enumerate();
        let mut last_index = 0;

        let mut key = "";
        let mut value = Value::String(String::new());
        let mut log_string = LogString::default();

        loop {
            match state {
                ParseState::StartLogLine => {
                    let end = match read_until(&mut iter, b',') {
                        Some(end) => end,
                        None => break,
                    };

                    let time = &data_str[last_index..end];
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
                    let nanos_pos = time
                        .as_bytes()
                        .iter()
                        .position(|char| *char == b'-')
                        .unwrap();

                    let minutes = match u32::from_str(&time[0..minutes_pos]) {
                        Ok(v) => v,
                        Err(_) => {
                            println!("{path} {last_index:x}");
                            panic!("FF")
                        }
                    };
                    let seconds = u32::from_str(&time[(minutes_pos + 1)..seconds_pos]).unwrap();
                    let nanos = &time[(seconds_pos + 1)..nanos_pos];
                    let nanos_count = nanos.chars().count();
                    let nanos = u32::from_str(nanos).unwrap();

                    log_string.time = Value::DateTime(match nanos_count {
                        0..=3 => NaiveDateTime::new(
                            date.date(),
                            NaiveTime::from_hms_milli(date.time().hour(), minutes, seconds, nanos),
                        ),
                        4..=6 => NaiveDateTime::new(
                            date.date(),
                            NaiveTime::from_hms_micro(date.time().hour(), minutes, seconds, nanos),
                        ),
                        _ => NaiveDateTime::new(
                            date.date(),
                            NaiveTime::from_hms_nano(date.time().hour(), minutes, seconds, nanos),
                        ),
                    });

                    log_string.duration = Value::from(&time[(nanos_pos + 1)..]);

                    state = ParseState::EventField;
                    last_index = end + 1;
                }
                ParseState::EventField => {
                    let end = read_until(&mut iter, b',').unwrap();
                    log_string.event = Value::from(&data_str[last_index..end]);

                    state = ParseState::Duration;
                    last_index = end + 1;
                }
                ParseState::Duration => {
                    let end = read_until(&mut iter, b',').unwrap();
                    // log_string.fields.insert(
                    //     "duration".to_string(),
                    //     String::from(&data_str[last_index..end]),
                    // );

                    state = ParseState::Key;
                    last_index = end + 1;
                }
                ParseState::Key => {
                    let end = read_until(&mut iter, b'=').unwrap();
                    key = &data_str[last_index..end];

                    state = ParseState::Value;
                    last_index = end + 1;
                }
                ParseState::Value => {
                    let mut value_state = ParseValueState::BeginParse;
                    loop {
                        match value_state {
                            ParseValueState::BeginParse => match iter.next() {
                                Some((begin, char))
                                if char == b',' || char == b'\r' || char == b'\n' =>
                                    {
                                        value = Value::String(String::new());
                                        value_state = ParseValueState::Finish((begin, char));
                                    }
                                Some((begin, char)) if char == b'\'' || char == b'"' => {
                                    last_index = begin + 1;
                                    value_state = ParseValueState::ReadValueUntil(char);
                                }
                                Some((begin, _)) => {
                                    last_index = begin;
                                    value_state = ParseValueState::ReadValueToNext;
                                }
                                None => unreachable!(),
                            },
                            ParseValueState::ReadValueUntil(quote) => {
                                let mut end = 0;
                                while let Some((index, char)) = iter.next() {
                                    match char {
                                        b'\'' | b'"' => {
                                            if data_str.as_bytes()[index + 1] == char {
                                                // Экранированная кавычка (пропускаем)
                                                iter.next().unwrap();
                                                continue;
                                            } else if char == quote {
                                                end = index;
                                                break;
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                value = Value::from(&data_str[last_index..end]);

                                let next = iter.next().unwrap();
                                value_state = ParseValueState::Finish((next.0, next.1));
                            }
                            ParseValueState::ReadValueToNext => {
                                while let Some((end, char)) = iter.next() {
                                    match char {
                                        b'\r' | b'\n' | b',' => {
                                            value = Value::from(&data_str[last_index..end]);

                                            value_state = ParseValueState::Finish((end, char));
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            ParseValueState::Finish((index, char)) => {
                                match char {
                                    b'\r' => {
                                        state = ParseState::Finish;
                                        iter.next().unwrap();
                                        last_index = index + 2;
                                        log_string.set_value(key, value.clone());
                                    }
                                    b'\n' => {
                                        state = ParseState::Finish;
                                        last_index = index + 1;
                                        log_string.set_value(key, value.clone());
                                    }
                                    b',' => {
                                        state = ParseState::Key;
                                        last_index = index + 1;
                                        log_string.set_value(key, value.clone());
                                    }
                                    _ => unreachable!(),
                                }
                                break;
                            }
                        }
                    }
                }
                ParseState::Finish => {
                    let mut tmp = LogString::default();
                    std::mem::swap(&mut tmp, &mut log_string);
                    sender.send(tmp).unwrap();
                    state = ParseState::StartLogLine
                }
            }
        }

        Ok(())
    }
}
