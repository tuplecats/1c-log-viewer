use crate::util::parse_time;
use chrono::{NaiveDate, NaiveDateTime, Timelike};
pub use compiler::{Compiler, Query};
pub use fields::*;
use indexmap::IndexMap;
use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io,
    io::{BufReader, Read, Seek, SeekFrom},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};
pub use value::*;
use walkdir::{DirEntry, WalkDir};

mod compiler;
mod fields;
pub mod logdata;
mod value;

#[derive(Debug, Clone)]
pub struct FieldMap<'a> {
    values: IndexMap<Cow<'a, str>, Value<'a>>,
}

impl<'a> FieldMap<'a> {
    pub fn new() -> FieldMap<'a> {
        FieldMap {
            values: IndexMap::with_capacity(16),
        }
    }

    pub fn insert<T: Into<Cow<'a, str>>>(&mut self, key: T, value: Value<'a>) {
        let key = key.into();

        if let Some(inner) = self.values.get_mut(&key) {
            match inner {
                Value::MultiValue(arr) => arr.push(value),
                _ => *inner = Value::MultiValue(vec![inner.clone(), value]),
            }
        } else {
            self.values.insert(key, value);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.values
            .iter()
            .flat_map(|(a, b)| b.iter().map(|b| (a.as_ref(), b)))
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<&Value> {
        self.values.get(name.as_ref())
    }

    pub fn get_index(&self, index: usize) -> Option<(String, &Value)> {
        let mut inner_index = 0;
        for value in self.values.iter() {
            if inner_index + value.1.len() > index {
                inner_index = index - inner_index;
                return Some((value.0.to_string(), &value.1[inner_index]));
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
    pub fn new(
        buf: Arc<Mutex<BufReader<File>>>,
        time: NaiveDateTime,
        begin: u64,
        size: u64,
    ) -> Self {
        Self {
            buf,
            time,
            begin,
            size,
        }
    }

    #[inline]
    pub fn begin(&self) -> u64 {
        self.begin
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.size as usize
    }

    pub fn fields(&self) -> Fields {
        Fields::new(self.to_string())
    }

    pub fn get(&self, name: &str) -> Option<Value<'static>> {
        match name {
            "time" => Some(Value::DateTime(self.time)),
            _ => {
                let f = self.fields();
                f.iter()
                    .find(|(k, _)| k == name)
                    .map(|(_, v)| Value::from(v.to_string()))
            }
        }
    }
}

impl ToString for LogString {
    fn to_string(&self) -> String {
        let mut lock = self.buf.lock().unwrap();
        lock.seek(SeekFrom::Start(self.begin() + 3)).unwrap();

        let mut data = vec![0; self.len()];
        lock.read_exact(&mut data).unwrap();
        unsafe { String::from_utf8_unchecked(data) }
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
    fn parse_dir(
        path: String,
        date: Option<NaiveDateTime>,
        sender: Sender<LogString>,
    ) -> io::Result<()> {
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
                        _ => Some((e, date_time)),
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        files.sort_by(|(_, name), (_, name2)| name.cmp(name2));

        let parts = files.into_iter().fold(
            Vec::<Vec<(DirEntry, NaiveDateTime)>>::new(),
            |mut acc, (entry, time)| {
                if acc.is_empty() {
                    acc.push(vec![]);
                } else if acc.last().unwrap().is_empty()
                    || acc.last().unwrap().last().unwrap().1 != time
                {
                    acc.push(vec![]);
                }

                acc.last_mut().unwrap().push((entry, time));
                acc
            },
        );

        for part in parts {
            let rows = part
                .into_iter()
                .map(|(entry, time)| {
                    let mut file = OpenOptions::new().read(true).open(entry.path()).unwrap();
                    file.seek(SeekFrom::Start(3)).unwrap();
                    let mut data = String::with_capacity(1024 * 30);
                    file.read_to_string(&mut data).unwrap();

                    (Arc::new(Mutex::new(BufReader::new(file))), data, time)
                })
                .filter(|(_, data, _)| !data.is_empty())
                .collect::<Vec<_>>();

            let mut part = rows
                .into_iter()
                .map(|(buf, data, hour)| (buf, Fields::new(data), hour))
                .collect::<Vec<_>>();

            let mut lines = vec![None; part.len()];
            loop {
                for (index, (buffer, data, hour)) in part.iter_mut().enumerate() {
                    if lines[index].is_some() {
                        continue;
                    }

                    loop {
                        let begin = data.current() as u64;
                        match data.parse_field() {
                            Some((key, value)) if key == "time" => {
                                let time = parse_time(*hour, &value);
                                match date {
                                    Some(date) if time < date => {}
                                    _ => {
                                        while let Some(_) = data.parse_field() {}
                                        let end = data.current() as u64;

                                        let line = LogString::new(
                                            buffer.clone(),
                                            time,
                                            begin,
                                            end - begin,
                                        );
                                        lines[index] = Some(line);
                                        break;
                                    }
                                }
                            }
                            Some(_) => unreachable!(),
                            None => break,
                        }
                    }
                }

                let min = lines
                    .iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        if let Some(value) = value.as_ref() {
                            Some((index, value))
                        } else {
                            None
                        }
                    })
                    .min_by(|(_, value1), (_, value2)| {
                        value1
                            .get("time")
                            .unwrap()
                            .partial_cmp(&value2.get("time").unwrap())
                            .unwrap()
                    })
                    .map(|(index, _)| index);

                if lines.iter().all(Option::is_none) {
                    break;
                }

                if let Some(min) = min {
                    let mut tmp = None;
                    std::mem::swap(&mut lines[min], &mut tmp);
                    sender.send(tmp.unwrap()).unwrap()
                }
            }
        }

        Ok(())
    }
}
