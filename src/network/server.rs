use std::collections::HashMap;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::protocol::{decode_message, encode_message, Message, Operation};

pub const CONTROL_PORT: u16 = 22223;

pub struct Member {
    pub id: u64,
    pub name: String,
    pub stream: TcpStream,
}

pub type MemberTable = Arc<Mutex<HashMap<u64, Member>>>;

/// 全局操作日志：分配 op_id 和全局顺序号 seq
pub struct GlobalLog {
    pub next_op_id: u64,
    pub next_seq: u64,
}

impl GlobalLog {
    pub fn new() -> Self {
        Self {
            next_op_id: 1,
            next_seq: 1,
        }
    }

    /// 分配一对 (op_id, seq)
    pub fn alloc(&mut self) -> (u64, u64) {
        let id = self.next_op_id;
        let seq = self.next_seq;
        self.next_op_id += 1;
        self.next_seq += 1;
        (id, seq)
    }
}

/// 启动 TCP Server
pub fn start_server(
    members: MemberTable,
    next_id: Arc<AtomicU64>,
    log: Arc<Mutex<GlobalLog>>,
) {
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
                    let log = log.clone();
                    thread::spawn(move || handle_client(stream, members, next_id, log));
                }
                Err(e) => {
                    crate::network::push_log(format!("accept 失败: {}", e));
                }
            }
        }
    });
}

fn handle_client(
    mut stream: TcpStream,
    members: MemberTable,
    next_id: Arc<AtomicU64>,
    log: Arc<Mutex<GlobalLog>>,
) {
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

    // 保存成员（复制一份 stream 用于写，原 stream 用于读）
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
            Ok(Message::OpRequest { mut op }) => {
                // 房主分配全局 id 和 seq
                let (op_id, seq) = {
                    let mut l = log.lock().unwrap();
                    l.alloc()
                };
                op.id = op_id;
                op.author = id;
                op.seq = seq;

                crate::network::push_log(format!(
                    "房主收到 OpRequest: id={}, seq={}, kind={:?}",
                    op_id, seq, op.kind
                ));

                // 房主自己也应用这个操作（通过队列，统一走定时器）
                crate::sync::queue::push(op.clone());

                // 广播 OpApply 给所有成员（包括发起者）
                broadcast(&members, None, &Message::OpApply { seq, op });
            }
            Ok(msg) => {
                crate::network::push_log(format!("收到成员 {} 的消息: {:?}", id, msg));
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

/// 房主本地发起操作：分配 id/seq，写队列，广播给所有成员
pub fn host_submit_op(
    op: Operation,
    members: &MemberTable,
    log: &Arc<Mutex<GlobalLog>>,
) {
    let (op_id, seq) = {
        let mut l = log.lock().unwrap();
        l.alloc()
    };
    let mut op = op;
    op.id = op_id;
    op.author = 0; // 房主自己的 author 用 0
    op.seq = seq;

    crate::network::push_log(format!(
        "房主本地操作: id={}, seq={}, kind={:?}",
        op_id, seq, op.kind
    ));

    // 房主自己也应用
    crate::sync::queue::push(op.clone());

    // 广播给所有成员
    broadcast(members, None, &Message::OpApply { seq, op });
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