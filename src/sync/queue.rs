use std::sync::Mutex;

use crate::protocol::Operation;

// 待应用的操作队列（远程传来或房主本地产生的 OpApply）
static PENDING_OPS: Mutex<Vec<Operation>> = Mutex::new(Vec::new());

pub fn push(op: Operation) {
    PENDING_OPS.lock().unwrap().push(op);
}

pub fn drain() -> Vec<Operation> {
    let mut q = PENDING_OPS.lock().unwrap();
    std::mem::take(&mut *q)
}