pub mod client;
pub mod discovery;
pub mod server;

use std::sync::Mutex;

static LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn push_log(msg: String) {
    LOG_BUFFER.lock().unwrap().push(msg);
}

pub fn drain_logs() -> Vec<String> {
    let mut buf = LOG_BUFFER.lock().unwrap();
    std::mem::take(&mut *buf)
}