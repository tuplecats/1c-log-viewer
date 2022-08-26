use crate::{
    parser::LogString,
    ui::{index::ModelIndex, model::DataModel},
};
use std::{
    borrow::Cow,
    sync::{Arc, mpsc::Receiver, RwLock},
};
use std::sync::mpsc::{Sender, TryRecvError};
use std::sync::{Mutex, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use crate::parser::{Compiler, Query};
use crate::parser::compiler::ParseError;
use crate::parser::value::Value;

struct Inner {
    lines: Vec<LogString>,
    filter: Option<Query>,
    mapping: Vec<usize>,
    notifier: Mutex<Sender<Option<Query>>>,
}

impl Inner {
    fn accept_row(&self, row: usize) -> bool {
        let line = match self.lines.get(row) {
            Some(line) => line,
            _ => unreachable!(),
        };

        if let Some(filter) = &self.filter {
            return filter.accept(&line.fields())
        }

        // Когда фильтр не указан, то строку принимаем всегда
        true
    }
}

pub struct LogCollection(Arc<RwLock<Inner>>);

impl Clone for LogCollection {
    fn clone(&self) -> Self {
        LogCollection(self.0.clone())
    }
}

impl LogCollection {
    pub fn new(receiver: Receiver<LogString>) -> LogCollection {

        let (notifier, rx) = std::sync::mpsc::channel();
        let this = LogCollection(Arc::new(RwLock::new(Inner {
            lines: vec![],
            filter: None,
            mapping: vec![],
            notifier: Mutex::new(notifier),
        })));

        let this_cloned = this.clone();
        std::thread::spawn(move || {
            while let Ok(data) = receiver.recv() {
                this_cloned.inner_mut().lines.push(data);
            }
        });

        let this_cloned = this.clone();
        std::thread::spawn(move || {
            let mut row = 0;
            loop {
                match rx.try_recv() {
                    Ok(filter) => {
                        let mut write = this_cloned.inner_mut();
                        write.filter = filter;
                        write.mapping.clear();
                        row = 0;
                    },
                    Err(TryRecvError::Disconnected) => {
                        break;
                    },
                    _ => {}
                }

                let rows = this_cloned.inner().lines.len();
                if row >= rows {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }

                let accept = this_cloned.inner().accept_row(row);
                if accept {
                    this_cloned.inner_mut().mapping.push(row)
                }

                row += 1;
            }
        });

        this
    }

    pub fn set_filter(&self, filter: String) -> Result<(), ParseError> {
        match Compiler::new().compile(filter.as_str()) {
            Ok(filter) =>  {
                self.inner_mut().notifier.lock().unwrap().send(Some(filter)).unwrap();
                Ok(())
            },
            Err(e) => Err(e)
        }
    }

    pub fn line(&self, row: usize) -> Option<LogString> {
        let this = self.inner();
        this.mapping.get(row).and_then(|i| this.lines.get(*i)).cloned()
    }

    fn inner(&self) -> RwLockReadGuard<'_, Inner> {
        self.0.read().unwrap()
    }

    fn inner_mut(&self) -> RwLockWriteGuard<'_, Inner> {
        self.0.write().unwrap()
    }
}

impl DataModel for LogCollection {
    fn rows(&self) -> usize {
        self.inner().mapping.len()
    }

    fn cols(&self) -> usize {
        5
    }

    fn header_index(&self, name: &str) -> Option<usize> {
        match name {
            "time" => Some(0),
            "event" => Some(1),
            "duration" => Some(2),
            "process" => Some(3),
            "OSThread" => Some(4),
            _ => None,
        }
    }

    fn header_data(&self, column: usize) -> Option<Cow<'_, str>> {
        match column {
            0 => Some(Cow::Borrowed("time")),
            1 => Some(Cow::Borrowed("event")),
            2 => Some(Cow::Borrowed("duration")),
            3 => Some(Cow::Borrowed("process")),
            4 => Some(Cow::Borrowed("OSThread")),
            _ => None,
        }
    }

    fn data(&self, index: ModelIndex) -> Option<Value> {
        let this = self.inner();
        let line = this.mapping.get(index.row()).and_then(|i| this.lines.get(*i));

        match (line, index.column()) {
            (Some(line), 0) => Some(line.get("time").unwrap_or_default()),
            (Some(line), 1) => Some(line.get("event").unwrap_or_default()),
            (Some(line), 2) => Some(line.get("duration").unwrap_or_default()),
            (Some(line), 3) => Some(line.get("process").unwrap_or_default()),
            (Some(line), 4) => Some(line.get("OSThread").unwrap_or_default()),
            _ => None,
        }
    }
}