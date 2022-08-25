use std::{
    io,
    io::Read,
    str::FromStr,
    sync::mpsc::{channel, Receiver, Sender},
};
use std::fs::OpenOptions;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use indexmap::IndexMap;
use walkdir::{DirEntry, WalkDir};

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
    Finish,
}

struct LineIter {
    data: String,
    date: NaiveDateTime,
    last_index: usize,
}

impl LineIter {
    fn new(data: String, date: NaiveDateTime) -> Self {
        LineIter {
            data,
            date,
            last_index: 0,
        }
    }

    fn parse_line(&mut self) -> Option<LogString> {
        let mut state = ParseState::StartLogLine;
        let mut key = "";
        let mut value = Value::String(String::new());
        let mut log_string = LogString::default();

        loop {
            match state {
                ParseState::StartLogLine => {
                    let size = match self.data[self.last_index..].as_bytes().iter().position(|&byte| byte == b',') {
                        Some(size) => size,
                        None => break,
                    };

                    let time = &self.data[self.last_index..(self.last_index + size)];
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
                        Err(_) => unreachable!()
                    };
                    let seconds = u32::from_str(&time[(minutes_pos + 1)..seconds_pos]).unwrap();
                    let nanos = &time[(seconds_pos + 1)..nanos_pos];
                    let nanos_count = nanos.chars().count();
                    let nanos = u32::from_str(nanos).unwrap();

                    log_string.time = Value::DateTime(match nanos_count {
                        0..=3 => NaiveDateTime::new(
                            self.date.date(),
                            NaiveTime::from_hms_milli(self.date.time().hour(), minutes, seconds, nanos),
                        ),
                        4..=6 => NaiveDateTime::new(
                            self.date.date(),
                            NaiveTime::from_hms_micro(self.date.time().hour(), minutes, seconds, nanos),
                        ),
                        _ => NaiveDateTime::new(
                            self.date.date(),
                            NaiveTime::from_hms_nano(self.date.time().hour(), minutes, seconds, nanos),
                        ),
                    });

                    log_string.duration = Value::from(&time[(nanos_pos + 1)..]);

                    state = ParseState::EventField;
                    self.last_index += size + 1;
                }
                ParseState::EventField => {
                    let size = self.data[self.last_index..].as_bytes().iter().position(|&byte| byte == b',').unwrap();
                    log_string.event = Value::from(&self.data[self.last_index..(self.last_index + size)]);

                    state = ParseState::Duration;
                    self.last_index += size + 1;
                }
                ParseState::Duration => {
                    let size = self.data[self.last_index..].as_bytes().iter().position(|&byte| byte == b',').unwrap();
                    // log_string.fields.insert(
                    //     "duration".to_string(),
                    //     String::from(&data_str[last_index..end]),
                    // );

                    state = ParseState::Key;
                    self.last_index += size + 1;
                }
                ParseState::Key => {
                    let size = self.data[self.last_index..].as_bytes().iter().position(|&byte| byte == b'=').unwrap();
                    key = &self.data[self.last_index..(self.last_index + size)];

                    state = ParseState::Value;
                    self.last_index += size + 1;
                }
                ParseState::Value => {
                    let mut value_state = ParseValueState::BeginParse;
                    loop {
                        match value_state {
                            ParseValueState::BeginParse => match self.data.as_bytes().get(self.last_index) {
                                Some(&char)
                                if char == b',' || char == b'\r' || char == b'\n' =>
                                    {
                                        value = Value::String(String::new());
                                        value_state = ParseValueState::Finish;
                                    }
                                Some(&char) if char == b'\'' || char == b'"' => {
                                    self.last_index += 1;
                                    value_state = ParseValueState::ReadValueUntil(char);
                                }
                                Some(_) => {
                                    value_state = ParseValueState::ReadValueToNext;
                                }
                                None => unreachable!(),
                            },
                            ParseValueState::ReadValueUntil(quote) => {
                                let mut end = self.last_index;
                                while let Some(&char) = self.data.as_bytes().get(end) {
                                    match char {
                                        b'\'' | b'"' => {
                                            if self.data.as_bytes()[end + 1] == char {
                                                // Экранированная кавычка (пропускаем)
                                                end += 1;
                                            } else if char == quote {
                                                break;
                                            }
                                        }
                                        _ => {}
                                    }
                                    end += 1;
                                }

                                value = Value::from(&self.data[self.last_index..end]);

                                self.last_index = end + 1;
                                value_state = ParseValueState::Finish;
                            }
                            ParseValueState::ReadValueToNext => {
                                let mut end = self.last_index;
                                while let Some(&char) = self.data.as_bytes().get(end) {
                                    match char {
                                        b'\r' | b'\n' | b',' => {
                                            value = Value::from(&self.data[self.last_index..end]);

                                            self.last_index = end;
                                            value_state = ParseValueState::Finish;
                                            break;
                                        }
                                        _ => {}
                                    }
                                    end += 1;
                                }
                            }
                            ParseValueState::Finish => {
                                match self.data.as_bytes().get(self.last_index) {
                                    Some(b'\r') => {
                                        state = ParseState::Finish;
                                        self.last_index += 2;
                                        log_string.set_value(key, value.clone());
                                    }
                                    Some(b'\n') => {
                                        state = ParseState::Finish;
                                        self.last_index += 1;
                                        log_string.set_value(key, value.clone());
                                    }
                                    Some(b',') => {
                                        state = ParseState::Key;
                                        self.last_index += 1;
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
                    return Some(log_string);
                }
            }
        }

        None
    }
}


impl Iterator for LineIter {
    type Item = LogString;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last_index >= self.data.len() {
            return None
        }

        self.parse_line()
    }
}

pub struct LogParser;

impl LogParser {
    pub fn parse(dir: String, date: Option<NaiveDateTime>) -> Receiver<LogString> {
        let (sender, receiver) = channel();
        std::thread::spawn(move || LogParser::parse_dir(dir, date, sender));
        receiver
    }

    // А может сделать итератор, который парсит
    fn parse_dir(path: String, date: Option<NaiveDateTime>, sender: Sender<LogString>) -> io::Result<()> {
        let walk = WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                !e.file_type().is_dir() && e.file_name().to_string_lossy().ends_with(".log")
            });

        let hour_date = date.map(|date| NaiveDate::from(date.date()).and_hms(date.hour(), 0, 0));
        let regex = regex::Regex::new(r#"^\d{8}[.]log$"#).unwrap();
        let mut files = walk
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if regex.is_match(&name) {
                    let year = 2000 + name[0..2].parse::<i32>().unwrap();
                    let month = name[2..4].parse::<u32>().unwrap();
                    let day = name[4..6].parse::<u32>().unwrap();
                    let hour = name[6..8].parse::<u32>().unwrap();

                    let date_time = NaiveDate::from_ymd(year, month, day).and_hms(hour, 0, 0);
                    match hour_date {
                        Some(hour_date) if date_time < hour_date => None,
                        _ =>  Some((e, date_time))
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        files.sort_by(|(_, name), (_, name2)| {
            name.cmp(name2)
        });

        let parts = files.into_iter()
            .fold(Vec::<Vec<(DirEntry, NaiveDateTime)>>::new(), |mut acc, (entry, time)| {
                if acc.is_empty() {
                    acc.push(vec![]);
                }
                else if acc.last().unwrap().is_empty() || acc.last().unwrap().last().unwrap().1 != time {
                    acc.push(vec![]);
                }

                acc.last_mut().unwrap().push((entry, time));
                acc
            });

        for part in parts {
            let mut part = part.into_iter()
                .map(|(entry, time)| {
                    let mut file = OpenOptions::new().read(true).open(entry.path()).unwrap();
                    let mut data = String::new();
                    file.read_to_string(&mut data).unwrap();
                    data.remove(0);
                    LineIter::new(data, time)
                })
                .collect::<Vec<_>>();

            let mut lines = vec![None; part.len()];
            loop {
                for (index, data) in part.iter_mut().enumerate() {
                    if lines[index].is_some() {
                        continue
                    }

                    loop {
                        match data.next() {
                            Some(line) => match date {
                                Some(date) if line.time < date => {},
                                _ => {
                                    lines[index] = Some(line);
                                    break
                                }
                            },
                            None => break
                        }
                    }
                }

                let min = lines.iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        if let Some(value) = value.as_ref() {
                            Some((index, value))
                        }
                        else {
                            None
                        }
                    })
                    .min_by(|(_, value1), (_, value2)| {
                        value1.time.partial_cmp(&value2.time).unwrap()
                    })
                    .map(|(index, _)| index);

                if lines.iter().all(Option::is_none) {
                    break
                }

                if let Some(min) = min {
                    let mut tmp = None;
                    std::mem::swap(&mut lines[min], &mut tmp);
                    sender.send(tmp.unwrap()).unwrap();
                }
            }
        }

        Ok(())
    }
}