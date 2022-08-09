use crate::parser::LogString;
use crate::ui::index::ModelIndex;
use crate::ui::model::TableModel;
use std::borrow::Cow;
use std::io::Read;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct LogData {
    lines: Arc<RwLock<Vec<LogString>>>,
    headers: Vec<String>,
}

impl Clone for LogData {
    fn clone(&self) -> Self {
        Self {
            lines: self.lines.clone(),
            headers: self.headers.clone(),
        }
    }
}

impl LogData {
    pub fn new(rx: Receiver<LogString>, headers: Vec<String>) -> Self {
        let this = Self {
            lines: Arc::new(RwLock::new(vec![])),
            headers,
        };

        let lines = this.lines.clone();
        std::thread::spawn(move || {
            while let Ok(data) = rx.recv() {
                match lines.write() {
                    Ok(mut lines) => lines.push(data),
                    Err(_) => println!("Error"),
                }
            }
        });
        this
    }

    fn lines(&self) -> RwLockReadGuard<'_, Vec<LogString>> {
        self.lines.read().unwrap()
    }

    fn lines_mut(&self) -> RwLockWriteGuard<'_, Vec<LogString>> {
        self.lines.write().unwrap()
    }
}

impl TableModel for LogData {
    fn rows(&self) -> usize {
        self.lines().len()
    }

    fn cols(&self) -> usize {
        self.headers.len()
    }

    fn header_index(&self, name: &str) -> Option<usize> {
        self.headers.iter().position(|header| header.eq(name))
    }

    fn header_data(&self, column: usize) -> Option<Cow<'_, str>> {
        match self.headers.get(column) {
            Some(column) => Some(Cow::Borrowed(column.as_str())),
            _ => None,
        }
    }

    fn data(&self, index: ModelIndex) -> Option<Cow<'_, str>> {
        let lines = self.lines();
        let (name, row) = (
            self.headers.get(index.column()).map(String::as_str),
            lines.get(index.row()),
        );

        match (name, row) {
            (Some("time"), Some(row)) => {
                Some(Cow::Owned(row.time.format("%d-%m-%y %T%.9f").to_string()))
            }
            (Some(name), Some(row)) => Some(Cow::Owned(
                row.fields.get(name).cloned().unwrap_or_default(),
            )),
            _ => None,
        }
    }
}
