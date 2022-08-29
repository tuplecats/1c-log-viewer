use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex, RwLock},
};

lazy_static::lazy_static! {
    static ref BUFFERS: RwLock<Vec<Arc<Mutex<BufReader<File>>>>> = RwLock::new(Vec::new());
}

#[inline]
pub(super) fn add_buffer(buffer: BufReader<File>) -> usize {
    let mut lock = BUFFERS.write().unwrap();
    lock.push(Arc::new(Mutex::new(buffer)));
    lock.len() - 1
}

#[inline]
pub(super) fn get_buffer(index: usize) -> Arc<Mutex<BufReader<File>>> {
    let lock = BUFFERS.read().unwrap();
    lock.get(index).cloned().unwrap()
}
