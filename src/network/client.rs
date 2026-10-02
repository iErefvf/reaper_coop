use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;

use crate::network::server::CONTROL_PORT;
use crate::protocol::{decode_message, encode_message, Message};

/// 客户端持有的与房主的连接
pub struct ClientConnection {
    pub member_id: u64,
    pub stream: TcpStream,
}

/// 连接房主，返回连接对象
pub fn join_room(host_ip: &str, name: &str) -> Result<ClientConnection, String> {
    let address = format!("{}:{}", host_ip, CONTROL_PORT)
        .to_socket_addrs()
        .map_err(|e| format!("解析房主地址失败: {}", e))?
        .next()
        .ok_or_else(|| "房主地址没有可用的 IP".to_string())?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .map_err(|e| format!("连接房主失败: {}", e))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("设置连接超时失败: {}", e))?;

    let req = Message::JoinRequest {
        name: name.to_string(),
        vst_list: vec![],
    };
    stream
        .write_all(&encode_message(&req))
        .map_err(|e| format!("发送 JoinRequest 失败: {}", e))?;

    let msg = decode_message(&mut stream)
        .map_err(|e| format!("接收 JoinAccept 失败: {}", e))?;
    let member_id = match msg {
        Message::JoinAccept { member_id, .. } => member_id,
        other => return Err(format!("预期 JoinAccept，收到 {:?}", other)),
    };
    stream
        .set_read_timeout(None)
        .map_err(|e| format!("清除连接超时失败: {}", e))?;

    crate::network::push_log(format!("已连接房主，成员 ID={}", member_id));

    // 启动读线程
    let mut read_stream = stream
        .try_clone()
        .map_err(|e| format!("复制 stream 失败: {}", e))?;
    thread::spawn(move || loop {
        match decode_message(&mut read_stream) {
            Ok(Message::OpApply { seq, op }) => {
                crate::network::push_log(format!(
                    "成员收到 OpApply: seq={}, kind={:?}",
                    seq, op.kind
                ));
                crate::sync::queue::push(op);
            }
            Ok(msg) => {
                crate::network::push_log(format!("收到房主消息: {:?}", msg));
            }
            Err(_) => {
                crate::network::push_log("与房主断开连接".to_string());
                break;
            }
        }
    });

    Ok(ClientConnection { member_id, stream })
}