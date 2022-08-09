use crate::ui::index::ModelIndex;
use crate::ui::model::TableModel;
use regex::Regex;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::mpsc::{Sender, TryRecvError};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::JoinHandle;
use std::time::Duration;

struct DataModelFilterInner {
    inner: Arc<dyn TableModel>,
    filter: HashMap<String, Regex>,
    mapping: Vec<usize>,

    stop: Option<Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl DataModelFilterInner {
    fn accept_row(&self, row: usize) -> bool {
        for (name, expr) in self.filter.iter() {
            match self.inner.header_index(name.as_str()) {
                Some(index) => match self.inner.data(ModelIndex::new(row, index)) {
                    Some(data) if expr.is_match(data.as_ref()) => {}
                    _ => return false,
                },
                None => return false,
            }
        }

        return true;
    }
}

unsafe impl Send for DataModelFilterInner {}
unsafe impl Sync for DataModelFilterInner {}

pub struct DataModelFilter(Arc<RwLock<DataModelFilterInner>>);

impl Clone for DataModelFilter {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl DataModelFilter {
    pub fn new(inner: Arc<dyn TableModel>, filter: HashMap<String, Regex>) -> Self {
        let object = Self(Arc::new(RwLock::new(DataModelFilterInner {
            inner,
            filter: HashMap::new(),
            mapping: vec![],
            stop: None,
            join: None,
        })));

        object.set_filter(filter);
        object
    }

    fn inner(&self) -> RwLockReadGuard<'_, DataModelFilterInner> {
        self.0.read().unwrap()
    }

    fn inner_mut(&self) -> RwLockWriteGuard<'_, DataModelFilterInner> {
        self.0.write().unwrap()
    }

    pub fn set_filter(&self, filter: HashMap<String, Regex>) {
        {
            let mut inner_mut = self.inner_mut();
            if let (Some(tx), Some(join)) = (inner_mut.stop.take(), inner_mut.join.take()) {
                tx.send(()).expect("stop thread");
                join.join().unwrap();
            }
            inner_mut.filter = filter;
        }

        self.filter_changed();
    }

    fn filter_changed(&self) {
        let rx = {
            let mut this = self.inner_mut();
            this.mapping.clear();

            let (tx, rx) = std::sync::mpsc::channel();
            this.stop = Some(tx);
            rx
        };

        let cloned = self.clone();
        std::thread::spawn(move || {
            let mut row = 0;
            loop {
                match rx.try_recv() {
                    Ok(()) | Err(TryRecvError::Disconnected) => break,
                    _ => {}
                }

                let rows = cloned.inner().inner.rows().saturating_sub(1);
                if row >= rows {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }

                let accept = cloned.inner().accept_row(row);
                if accept {
                    cloned.inner_mut().mapping.push(row)
                }

                row += 1;
            }
        });
    }
}

impl TableModel for DataModelFilter {
    fn rows(&self) -> usize {
        let this = self.inner();
        if !this.filter.is_empty() {
            this.mapping.len()
        } else {
            this.inner.rows()
        }
    }

    fn cols(&self) -> usize {
        self.inner().inner.cols()
    }

    fn header_index(&self, name: &str) -> Option<usize> {
        self.inner().inner.header_index(name)
    }

    fn header_data(&self, column: usize) -> Option<Cow<'_, str>> {
        self.inner()
            .inner
            .header_data(column)
            .map(|v| Cow::Owned(v.to_string()))
    }

    fn data(&self, index: ModelIndex) -> Option<Cow<'_, str>> {
        let this = self.inner();
        let source_index = match this.filter.is_empty() {
            true => index,
            false => ModelIndex::new(this.mapping[index.row()], index.column()),
        };

        this.inner
            .data(source_index)
            .map(|v| Cow::Owned(v.to_string()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
