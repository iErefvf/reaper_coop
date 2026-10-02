use reaper_medium::{CommandId, HookCommand};

use crate::protocol::{OpKind, Operation};

// 拦截普通 REAPER Action，包括快捷键和右键菜单命令。
pub struct CoopHook;

impl HookCommand for CoopHook {
    fn call(command_id: CommandId, _flag: i32) -> bool {
        // REAPER 插入轨道命令 ID 40001（末尾插入）
        if command_id.get() == 40001 {
            crate::network::push_log("拦截插入轨道命令".to_string());
            let op = Operation {
                id: 0,
                seq: 0,
                author: 0,
                kind: OpKind::TrackAdd { index: -1 },
                undo_data: vec![],
            };
            crate::sync::send_op(op);
            return true; // 阻止 REAPER 原生执行
        }
        false
    }
}