use serde::{Serialize, Deserialize};
use std::io::{self, Read};

pub fn encode_message(message: &Message) -> Vec<u8> {
    let payload = serde_json::to_vec(message).expect("Message serialization should not fail");
    let length = u32::try_from(payload.len()).expect("Message is too large to encode");

    let mut encoded = Vec::with_capacity(4 + payload.len());
    encoded.extend_from_slice(&length.to_be_bytes());
    encoded.extend_from_slice(&payload);
    encoded
}

pub fn decode_message(reader: &mut impl Read) -> io::Result<Message> {
    let mut length_bytes = [0; 4];
    reader.read_exact(&mut length_bytes)?;
    let length = u32::from_be_bytes(length_bytes) as usize;

    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    serde_json::from_slice(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    // 握手
    JoinRequest { name: String, vst_list: Vec<String> },
    JoinAccept { member_id: u64, room_name: String, vst_missing: Vec<String> },
    MemberJoin { member_id: u64, name: String },
    MemberLeave { member_id: u64 },

    // 操作同步
    OpRequest { op: Operation },
    OpApply { seq: u64, op: Operation },

    // 撤销
    UndoRequest { target_op_id: u64 },
    UndoApply { seq: u64, target_op_id: u64, undo_op: Operation },
    UndoReject { target_op_id: u64, reason: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Operation {
    pub id: u64,
    pub author: u64,
    pub kind: OpKind,
    pub undo_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OpKind {
    TrackAdd { index: i32 },
    TrackDelete { track_guid: String },
    // 后续逐步添加
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let msg = Message::JoinRequest {
            name: "测试".to_string(),
            vst_list: vec!["ReaEQ".to_string()],
        };
        let encoded = encode_message(&msg);
        let mut cursor = std::io::Cursor::new(encoded);
        let decoded = decode_message(&mut cursor).unwrap();
        match decoded {
            Message::JoinRequest { name, .. } => assert_eq!(name, "测试"),
            _ => panic!("类型不匹配"),
        }
    }
}