pub mod client;
pub mod discovery;
pub mod server;

use std::net::{TcpListener, UdpSocket};
use std::sync::Mutex;

use crate::network::discovery::DISCOVERY_PORT;
use crate::network::server::CONTROL_PORT;

// 全局日志缓冲：后台线程往里 push，主线程通过 Action 读取
static LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn push_log(msg: String) {
    LOG_BUFFER.lock().unwrap().push(msg);
}

pub fn drain_logs() -> Vec<String> {
    let mut buf = LOG_BUFFER.lock().unwrap();
    std::mem::take(&mut *buf)
}

/// 在插件初始化时检查网络接口和协作端口，避免等到创建/搜索房间时才失败。
pub fn initialize() -> Result<(), String> {
    let ip = discovery::local_ip();

    let tcp_listener = TcpListener::bind(("0.0.0.0", CONTROL_PORT))
        .map_err(|e| format!("TCP 端口 {} 不可用: {}", CONTROL_PORT, e))?;
    drop(tcp_listener);

    let udp_socket = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))
        .map_err(|e| format!("UDP 端口 {} 不可用: {}", DISCOVERY_PORT, e))?;
    drop(udp_socket);

    push_log(format!(
        "网络初始化完成，本机地址 {}，TCP={}，UDP={}",
        ip, CONTROL_PORT, DISCOVERY_PORT
    ));
    Ok(())
}