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
    pub room_type: u8,
    pub member_count: u32,
    pub host_ip: String,
}

pub fn local_ip() -> String {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind failed");
    let _ = socket.connect("8.8.8.8:80");
    socket
        .local_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

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
        let mut broadcast_targets = vec![SocketAddr::from((
            Ipv4Addr::BROADCAST,
            DISCOVERY_PORT,
        ))];
        if let Ok(ip) = room_info.host_ip.parse::<Ipv4Addr>() {
            let octets = ip.octets();
            let directed = SocketAddr::from((
                Ipv4Addr::new(octets[0], octets[1], octets[2], 255),
                DISCOVERY_PORT,
            ));
            if !broadcast_targets.contains(&directed) {
                broadcast_targets.push(directed);
            }
        }
        // 单机自测用：同时往本机回环发一份
        let local_target = SocketAddr::from((Ipv4Addr::LOCALHOST, DISCOVERY_PORT));

        crate::network::push_log(format!(
            "开始广播房间: {} ({})",
            room_info.room_name, room_info.host_ip
        ));

        while running.load(Ordering::Relaxed) {
            let json = serde_json::to_string(&room_info).unwrap_or_default();
            for target in &broadcast_targets {
                if let Err(e) = socket.send_to(json.as_bytes(), target) {
                    crate::network::push_log(format!("发送房间广播到 {} 失败: {}", target, e));
                }
            }
            if let Err(e) = socket.send_to(json.as_bytes(), local_target) {
                crate::network::push_log(format!("发送本机发现广播失败: {}", e));
            }
            thread::sleep(Duration::from_secs(3));
        }

        crate::network::push_log("房间广播已停止".to_string());
    });
}

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
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok();

        crate::network::push_log(format!(
            "开始监听局域网房间，端口 {}",
            DISCOVERY_PORT
        ));

        let mut buf = [0u8; 4096];
        while running.load(Ordering::Relaxed) {
            if let Ok((n, src)) = socket.recv_from(&mut buf) {
                if let Ok(mut info) = serde_json::from_slice::<RoomInfo>(&buf[..n]) {
                    // 以报文实际来源为准，避免多网卡时房主填入 VPN/虚拟网卡地址。
                    if let SocketAddr::V4(addr) = src {
                        info.host_ip = addr.ip().to_string();
                    }
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