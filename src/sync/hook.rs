use reaper_medium::{CommandId, HookCommand, HookPostCommand};

use crate::protocol::{OpKind, Operation};

fn send_track_add(change: crate::sync::observer::ActionChange) {
    let action_label = crate::sync::actions::find(change.command_id)
        .map(|action| {
            format!(
                "{} ({:?}, versions={})",
                action.name,
                action.category,
                action.versions.join(",")
            )
        })
        .unwrap_or_else(|| "未登记 Action".to_string());

    if !change.removed_track_guids.is_empty() || change.track_order_changed {
        crate::network::push_log(format!(
            "Action {} [{}] 轨道变化：{} -> {}，删除={}，顺序变化={}",
            change.command_id,
            action_label,
            change.before.tracks.len(),
            change.after.tracks.len(),
            change.removed_track_guids.len(),
            change.track_order_changed
        ));
    }
    if change.added_tracks.is_empty() {
        return;
    }
    if change
        .added_tracks
        .iter()
        .any(|track| track.chunk.is_none())
    {
        crate::network::push_log(format!(
            "Action {} 新增轨道，但无法完整读取轨道状态，已跳过同步",
            change.command_id
        ));
        return;
    }

    crate::network::push_log(format!(
        "Action {} [{}] 新增 {} 条轨道，开始同步完整轨道状态",
        change.command_id,
        action_label,
        change.added_tracks.len()
    ));
    let index = change
        .after
        .tracks
        .iter()
        .position(|track| {
            change
                .added_tracks
                .iter()
                .any(|added| added.guid == track.guid)
        })
        .unwrap_or(change.after.tracks.len()) as i32;
    crate::sync::send_op(Operation {
        id: 0,
        seq: 0,
        author: 0,
        applied_locally: true,
        kind: OpKind::TrackAdd {
            index,
            chunks: change
                .added_tracks
                .into_iter()
                .map(|track| track.chunk.expect("checked above"))
                .collect(),
        },
        undo_data: vec![],
    });
}

// 拦截普通 REAPER Action，包括快捷键和右键菜单命令。
pub struct CoopHook;

impl HookCommand for CoopHook {
    fn call(command_id: CommandId, _flag: i32) -> bool {
        // 不拦截原生 Action，保留其版本相关的插入位置、数量和模板交互。
        // 统一在执行前后比较轨道 GUID，因此不需要维护易过时的命令 ID 白名单。
        crate::sync::observer::before_action(command_id.get());
        false
    }
}

pub struct CoopPostHook;

impl HookPostCommand for CoopPostHook {
    fn call(command_id: CommandId, _flag: i32) {
        if let Some(change) = crate::sync::observer::after_action(command_id.get()) {
            send_track_add(change);
        }
    }
}