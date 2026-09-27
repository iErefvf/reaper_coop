use std::collections::HashMap;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::protocol::{decode_message, encode_message, Message};

pub const CONTROL_PORT: u16 = 22223;

pub struct Member {
    pub id: u64,
    pub name: String,
    pub stream: TcpStream,
}

pub type MemberTable = Arc<Mutex<HashMap<u64, Member>>>;

/// 启动 TCP Server，返回监听线程的句柄
pub fn start_server(members: MemberTable, next_id: Arc<AtomicU64>) {
    thread::spawn(move || {
        let listener = match TcpListener::bind(format!("0.0.0.0:{}", CONTROL_PORT)) {
            Ok(l) => l,
            Err(e) => {
                crate::network::push_log(format!("TCP Server 绑定失败: {}", e));
                return;
            }
        };
        crate::network::push_log(format!("TCP Server 已启动，端口 {}", CONTROL_PORT));

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let members = members.clone();
                    let next_id = next_id.clone();
                    thread::spawn(move || handle_client(stream, members, next_id));
                }
                Err(e) => {
                    crate::network::push_log(format!("accept 失败: {}", e));
                }
            }
        }
    });
}

fn handle_client(mut stream: TcpStream, members: MemberTable, next_id: Arc<AtomicU64>) {
    // 读取第一条消息，必须是 JoinRequest
    let first = match decode_message(&mut stream) {
        Ok(msg) => msg,
        Err(e) => {
            crate::network::push_log(format!("读取握手消息失败: {}", e));
            return;
        }
    };

    let (name, _vst_list) = match first {
        Message::JoinRequest { name, vst_list } => (name, vst_list),
        _ => {
            crate::network::push_log("第一条消息不是 JoinRequest，断开".to_string());
            return;
        }
    };

    // 分配 member_id
    let id = next_id.fetch_add(1, Ordering::Relaxed);

    // 保存成员
    let stream_clone = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            crate::network::push_log(format!("复制 stream 失败: {}", e));
            return;
        }
    };
    {
        let mut table = members.lock().unwrap();
        table.insert(
            id,
            Member {
                id,
                name: name.clone(),
                stream: stream_clone,
            },
        );
    }
    crate::network::push_log(format!("成员 '{}' 加入，ID={}", name, id));

    // 回复 JoinAccept
    let accept = Message::JoinAccept {
        member_id: id,
        room_name: "我的房间".to_string(),
        vst_missing: vec![],
    };
    if stream.write_all(&encode_message(&accept)).is_err() {
        crate::network::push_log(format!("发送 JoinAccept 给 {} 失败", id));
    }

    // 广播 MemberJoin 给其他成员
    broadcast(
        &members,
        Some(id),
        &Message::MemberJoin {
            member_id: id,
            name: name.clone(),
        },
    );

    // 进入读循环
    loop {
        match decode_message(&mut stream) {
            Ok(msg) => {
                crate::network::push_log(format!("收到成员 {} 的消息: {:?}", id, msg));
                // 阶段 5 在这里处理 OpRequest / UndoRequest
            }
            Err(_) => {
                crate::network::push_log(format!("成员 {} ({}) 断开", id, name));
                members.lock().unwrap().remove(&id);
                broadcast(&members, None, &Message::MemberLeave { member_id: id });
                break;
            }
        }
    }
}

/// 广播消息给所有成员；exclude 为 Some(id) 时排除该成员
pub fn broadcast(members: &MemberTable, exclude: Option<u64>, msg: &Message) {
    let data = encode_message(msg);
    let mut table = members.lock().unwrap();
    let mut dead = Vec::new();
    for (id, m) in table.iter_mut() {
        if Some(*id) == exclude {
            continue;
        }
        if m.stream.write_all(&data).is_err() {
            dead.push(*id);
        }
    }
    for id in dead {
        table.remove(&id);
    }
}