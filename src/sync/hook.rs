use reaper_medium::{
    ActionValueChange, CommandId, HookCommand2, SectionContext, WindowContext,
};

use crate::protocol::{OpKind, Operation};

// 实现 HookCommand2 trait 的处理器
pub struct CoopHook;

impl HookCommand2 for CoopHook {
    fn call(
        _section: SectionContext,
        command_id: CommandId,
        _value_change: ActionValueChange,
        _window: WindowContext,
    ) -> bool {
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