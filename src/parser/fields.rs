
use std::cell::Cell;
use std::borrow::Cow;
use crate::parser::{FieldMap, Value};

#[derive(Clone, Copy)]
enum ParseState {
    StartLogLine,
    Duration,
    EventField,
    Undefined,
    Key,
    Value,
    Finish,
}

#[derive(PartialEq)]
enum ParseValueState {
    BeginParse,
    ReadValueUntil(u8),
    ReadValueToNext,
    Finish(u8),
}

pub struct Fields {
    reader: String,
    state: Cell<ParseState>,
    index: Cell<usize>,
}

impl Fields {
    pub fn new(reader: String) -> Self {
        Fields {
            reader,
            state: Cell::new(ParseState::StartLogLine),
            index: Cell::new(0),
        }
    }

    pub fn current(&self) -> usize {
        self.index.get()
    }

    fn read_until(&self, find: u8) -> Option<&str> {
        let begin = self.index.get();
        let mut size = 0 as usize;
        while let Some(byte) = self.read_byte() {
            size += 1;

            if byte == find {
                break;
            }
        }

        let size = size.saturating_sub(1);
        match size {
            0 => None,
            _ => Some(&self.reader[begin..(begin + size)])
        }
    }

    fn read_byte(&self) -> Option<u8> {
        if self.index.get() == self.reader.len() {
            return None
        }

        self.index.set(self.index.get().saturating_add(1).min(self.reader.len()));
        Some(self.reader.as_bytes()[self.index.get() - 1])
    }

    fn read_value(&self) -> Option<&str> {

        let mut value = "";
        let mut value_state = ParseValueState::BeginParse;

        loop {
            match value_state {
                ParseValueState::BeginParse => match self.read_byte() {
                    Some(char) if char == b'\r' || char == b'\n' || char == b',' => {
                        value = "";
                        value_state = ParseValueState::Finish(char);
                    }
                    Some(char) if char == b'\'' || char == b'"' => {
                        value_state = ParseValueState::ReadValueUntil(char);
                    }
                    Some(_) => {
                        value_state = ParseValueState::ReadValueToNext;
                    }
                    None => unreachable!(),
                },
                ParseValueState::ReadValueUntil(quote) => {
                    let begin = self.current();
                    while let Some(char) = self.read_byte() {
                        match char {
                            b'\'' | b'"' if char == quote => {
                                let end = self.current().saturating_sub(1);
                                let read = self.read_byte();
                                match read {
                                    Some(byte) if char == byte => {
                                        continue
                                    },
                                    _ => {}
                                };

                                value = &self.reader[begin..end];
                                value_state = ParseValueState::Finish(read.unwrap());
                                break;
                            },
                            _ => {}
                        }
                    }
                }
                ParseValueState::ReadValueToNext => {
                    let begin = self.current().saturating_sub(1);
                    while let Some(char) = self.read_byte() {
                        match char {
                            b'\r' | b'\n' | b',' => {
                                value = &self.reader[begin..self.current().saturating_sub(1)];
                                value_state = ParseValueState::Finish(char);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                ParseValueState::Finish(char) => {
                    match char {
                        b'\r' => {
                            self.read_byte()?; //read n
                            self.state.set(ParseState::Finish);
                        }
                        b'\n' => {
                            self.state.set(ParseState::Finish);
                        }
                        b',' => {
                            self.state.set(ParseState::Key);
                        }
                        _ => unreachable!(),
                    }
                    break;
                }
            }
        }

        Some(value)
    }

    pub fn parse_field<'a>(&'a self) -> Option<(Cow<'a, str>, &str)> {
        let mut key = "";
        let value;

        loop {
            match self.state.get() {
                ParseState::StartLogLine => {
                    let value = self.read_until(b'-')?;
                    self.state.set(ParseState::Duration);
                    return Some((Cow::Borrowed("time"), value));
                }
                ParseState::Duration => {
                    let value = self.read_until(b',')?;
                    self.state.set(ParseState::EventField);
                    return Some((Cow::Borrowed("duration"), value));
                }
                ParseState::EventField => {
                    let value = self.read_until(b',')?;
                    self.state.set(ParseState::Undefined);
                    return Some((Cow::Borrowed("event"), value));
                }
                ParseState::Undefined => {
                    let _ = self.read_until(b',')?;
                    self.state.set(ParseState::Key);
                }
                ParseState::Key => {
                    key = self.read_until(b'=')?;
                    self.state.set(ParseState::Value);
                }
                ParseState::Value => {
                    value = self.read_value()?;
                    return Some((Cow::Borrowed(key), value));
                }
                ParseState::Finish => {
                    self.state.set(ParseState::StartLogLine);
                    break
                }
            }
        }

        None
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter { inner: self }
    }
}

pub struct Iter<'a> {
    inner: &'a Fields
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Cow<'a, str>, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.parse_field()
    }
}

impl From<Fields> for FieldMap<'static> {
    fn from(iter: Fields) -> Self {
        let mut map = FieldMap::new();
        while let Some((k, v)) = iter.parse_field() {
            map.insert(k.to_string(), Value::from(v.to_string()))
        }
        map
    }
}