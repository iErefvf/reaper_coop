use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const DISCOVERY_PORT: u16 = 22222;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RoomInfo {
    pub room_id: String,
    pub room_name: String,
    pub room_type: u8,       // 1=自由, 2=管理, 3=隔离
    pub member_count: u32,
    pub host_ip: String,
}

/// 获取本机在局域网中的 IP
pub fn local_ip() -> String {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind failed");
    // UDP connect 不会真正发包，只让操作系统选择出站网卡
    let _ = socket.connect("8.8.8.8:80");
    socket
        .local_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// 房主端：周期性广播房间信息
pub fn start_broadcast(room_info: RoomInfo, running: Arc<AtomicBool>) {
    thread::spawn(move || {
        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                crate::network::push_log(format!("广播 socket 绑定失败: {}", e));
                return;
            }
        };
        if socket.set_broadcast(true).is_err() {
            crate::network::push_log("设置广播模式失败".to_string());
            return;
        }
        //let target = SocketAddr::from((Ipv4Addr::BROADCAST, DISCOVERY_PORT));

        crate::network::push_log(format!(
            "开始广播房间: {} ({})",
            room_info.room_name, room_info.host_ip
        ));

        let broadcast_target = SocketAddr::from((Ipv4Addr::BROADCAST, DISCOVERY_PORT));
        let local_target = SocketAddr::from((Ipv4Addr::LOCALHOST, DISCOVERY_PORT));

        while running.load(Ordering::Relaxed) {
            let json = serde_json::to_string(&room_info).unwrap_or_default();
            let _ = socket.send_to(json.as_bytes(), broadcast_target);
            let _ = socket.send_to(json.as_bytes(), local_target);
            thread::sleep(Duration::from_secs(3));
        }

        crate::network::push_log("房间广播已停止".to_string());
    });
}

/// 成员端：监听广播，收集房间列表
pub fn start_discovery(
    rooms: Arc<Mutex<HashMap<String, RoomInfo>>>,
    running: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let socket = match UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)) {
            Ok(s) => s,
            Err(e) => {
                crate::network::push_log(format!("监听 socket 绑定失败: {}", e));
                return;
            }
        };
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .ok();

        crate::network::push_log(format!(
            "开始监听局域网房间，端口 {}",
            DISCOVERY_PORT
        ));

        let mut buf = [0u8; 4096];
        while running.load(Ordering::Relaxed) {
            if let Ok((n, _src)) = socket.recv_from(&mut buf) {
                if let Ok(info) = serde_json::from_slice::<RoomInfo>(&buf[..n]) {
                    crate::network::push_log(format!(
                        "发现房间: {} (host={}, 成员数={})",
                        info.room_name, info.host_ip, info.member_count
                    ));
                    rooms.lock().unwrap().insert(info.room_id.clone(), info);
                }
            }
        }

        crate::network::push_log("房间监听已停止".to_string());
    });
}