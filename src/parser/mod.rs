use std::{
    io,
    io::Read,
    str::FromStr,
    sync::mpsc::{channel, Receiver, Sender},
};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use indexmap::IndexMap;
use walkdir::{DirEntry, WalkDir};

mod compiler;
mod value;
pub mod logdata;

pub use value::*;
pub use compiler::{Compiler, Query};

#[derive(Debug, Clone)]
pub struct FieldMap {
    values: IndexMap<String, Value>,
}

impl FieldMap {
    pub fn new() -> FieldMap {
        FieldMap {
            values: IndexMap::with_capacity(16),
        }
    }

    pub fn insert<T: Into<String>>(&mut self, key: T, value: Value) {
        let key = key.into();

        if let Some(inner) = self.values.get_mut(&key) {
            match inner {
                Value::MultiValue(arr) => arr.push(value),
                _ => *inner = Value::MultiValue(vec![inner.clone(), value])
            }
        }
        else {
            self.values.insert(key, value);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.values.iter()
            .flat_map(|(a, b)| b.iter().map(|b| (a.as_str(), b)))
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<&Value> {
        self.values.get(name.as_ref())
    }

    pub fn get_index(&self, index: usize) -> Option<(&String, &Value)> {
        let mut inner_index = 0;
        for value in self.values.iter() {
            if inner_index + value.1.len() > index {
                inner_index = index - inner_index;
                return Some((value.0, &value.1[inner_index]))
            }

            inner_index += value.1.len();
        }
        None
    }

    pub fn len(&self) -> usize {
        self.values.iter().map(|(_, v)| v).map(Value::len).sum()
    }
}

#[derive(Debug, Clone)]
pub struct LogString {
    buf: Arc<Mutex<BufReader<File>>>,
    time: NaiveDateTime,
    begin: u64,
    size: u64,
}

impl LogString {
    pub fn new(buf: Arc<Mutex<BufReader<File>>>, time: NaiveDateTime, begin: u64, size: u64) -> Self  {
        Self {
            buf,
            time,
            begin,
            size,
        }
    }

    pub fn begin(&self) -> u64 {
        self.begin
    }

    pub fn len(&self) -> usize {
        self.size as usize
    }

    pub fn fields(&self) -> FieldMap {
        let mut lock = self.buf.lock().unwrap();
        lock.seek(SeekFrom::Start(self.begin + 3)).unwrap();
        let mut data = vec![0; self.size as usize];
        lock.read_exact(&mut data).unwrap();

        let mut iter = LineIter::new(self.buf.clone(), String::from_utf8(data).unwrap(), self.time, false);
        match iter.next().unwrap() {
            LogLine::Data(map) => map,
            _ => unreachable!()
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        match name {
            "time" => Some(Value::DateTime(self.time)),
            _ => self.fields().get(name).cloned()
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

#[derive(Debug, Clone)]
enum LogLine {
    Range(LogString),
    Data(FieldMap)
}

impl LogLine {
    pub fn get(&self, name: &str) -> Option<Value> {
        match self {
            LogLine::Range(l) => l.get(name),
            LogLine::Data(l) => l.get(name).cloned(),
        }
    }
}

struct LineIter {
    buffer: Arc<Mutex<BufReader<File>>>,
    data: String,
    date: NaiveDateTime,
    last_index: usize,
    parse_range: bool,
}

impl LineIter {
    fn new(buffer: Arc<Mutex<BufReader<File>>>, data: String, date: NaiveDateTime, parse_range: bool) -> Self {
        LineIter {
            buffer,
            data,
            date,
            last_index: 0,
            parse_range,
        }
    }

    fn parse_line(&mut self) -> Option<LogLine> {
        let mut state = ParseState::StartLogLine;
        let begin = self.last_index;
        let mut gtime = self.date;

        let mut key = "";
        let mut value = Value::String(String::new());
        let mut log_string = FieldMap::new();

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

                    gtime = match nanos_count {
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
                    };

                    if !self.parse_range {
                        log_string.insert("duration", Value::from(&time[(nanos_pos + 1)..]));
                    }

                    state = ParseState::EventField;
                    self.last_index += size + 1;
                }
                ParseState::EventField => {
                    let size = self.data[self.last_index..].as_bytes().iter().position(|&byte| byte == b',').unwrap();

                    log_string.insert("event", Value::from(&self.data[self.last_index..(self.last_index + size)]));

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

                                        if !self.parse_range {
                                            log_string.insert(key, value.clone());
                                        }
                                    }
                                    Some(b'\n') => {
                                        state = ParseState::Finish;
                                        self.last_index += 1;

                                        if !self.parse_range {
                                            log_string.insert(key, value.clone());
                                        }
                                    }
                                    Some(b',') => {
                                        state = ParseState::Key;
                                        self.last_index += 1;

                                        if !self.parse_range {
                                            log_string.insert(key, value.clone());
                                        }
                                    }
                                    _ => unreachable!(),
                                }
                                break;
                            }
                        }
                    }
                }
                ParseState::Finish => {
                    return match self.parse_range {
                        true => Some(LogLine::Range(LogString::new(self.buffer.clone(), gtime, begin as u64, (self.last_index - begin) as u64))),
                        false => Some(LogLine::Data(log_string))
                    };
                }
            }
        }

        None
    }
}


impl Iterator for LineIter {
    type Item = LogLine;

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
                    file.seek(SeekFrom::Start(3)).unwrap();
                    file.read_to_string(&mut data).unwrap();
                    LineIter::new(Arc::new(Mutex::new(BufReader::new(file))), data, time, true)
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
                                Some(date) if line.get("time").unwrap() < date => {},
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
                        value1.get("time").unwrap().partial_cmp(&value2.get("time").unwrap()).unwrap()
                    })
                    .map(|(index, _)| index);

                if lines.iter().all(Option::is_none) {
                    break
                }

                if let Some(min) = min {
                    let mut tmp = None;
                    std::mem::swap(&mut lines[min], &mut tmp);
                    match tmp {
                        Some(LogLine::Range(r)) => sender.send(r).unwrap(),
                        _ => unreachable!()
                    }
                }
            }
        }

        Ok(())
    }
}