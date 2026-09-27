pub mod discovery;

use std::sync::Mutex;

// 全局日志缓冲：后台线程往里 push，主线程通过 Action 读取
static LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn push_log(msg: String) {
    LOG_BUFFER.lock().unwrap().push(msg);
}

pub fn drain_logs() -> Vec<String> {
    let mut buf = LOG_BUFFER.lock().unwrap();
    std::mem::take(&mut *buf)
}