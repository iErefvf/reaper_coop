pub mod apply;
pub mod hook;
pub mod queue;

use std::sync::Mutex;

use crate::protocol::Operation;

// 全局操作发送通道
// 房主：交给 host_submit_op
// 成员：通过 TCP 发给房主
static OP_SENDER: Mutex<Option<Box<dyn Fn(Operation) + Send + Sync>>> = Mutex::new(None);

pub fn set_op_sender(sender: Box<dyn Fn(Operation) + Send + Sync>) {
    *OP_SENDER.lock().unwrap() = Some(sender);
}

pub fn send_op(op: Operation) {
    let guard = OP_SENDER.lock().unwrap();
    if let Some(sender) = guard.as_ref() {
        sender(op);
    } else {
        crate::network::push_log("OP_SENDER 未设置，操作被丢弃".to_string());
    }
}