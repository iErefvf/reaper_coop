use std::collections::HashSet;
use std::sync::Mutex;

use reaper_high::Reaper;
use reaper_medium::ChunkCacheHint;

#[derive(Clone, Debug)]
pub struct TrackSnapshot {
    pub guid: String,
    pub chunk: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ActionSnapshot {
    pub command_id: u32,
    pub tracks: Vec<TrackSnapshot>,
}

#[derive(Clone, Debug)]
pub struct ActionChange {
    pub command_id: u32,
    pub before: ActionSnapshot,
    pub after: ActionSnapshot,
    pub added_tracks: Vec<TrackSnapshot>,
    pub removed_track_guids: Vec<String>,
    pub track_order_changed: bool,
}

static PENDING_ACTIONS: Mutex<Vec<ActionSnapshot>> = Mutex::new(Vec::new());

pub fn capture(command_id: u32, include_chunks: bool) -> ActionSnapshot {
    let tracks = Reaper::get()
        .current_project()
        .tracks()
        .map(|track| {
            let guid = track.guid().to_string_with_braces();
            let chunk = if include_chunks {
                match track.chunk(64 * 1024 * 1024, ChunkCacheHint::UndoMode) {
                    Ok(value) => Some(value.to_string()),
                    Err(error) => {
                        crate::network::push_log(format!(
                            "读取轨道状态失败，GUID={}: {}",
                            guid, error
                        ));
                        None
                    }
                }
            } else {
                None
            };
            TrackSnapshot { guid, chunk }
        })
        .collect();

    ActionSnapshot { command_id, tracks }
}

pub fn before_action(command_id: u32) {
    PENDING_ACTIONS
        .lock()
        .unwrap()
        .push(capture(command_id, false));
}

pub fn after_action(command_id: u32) -> Option<ActionChange> {
    let before = PENDING_ACTIONS.lock().unwrap().pop()?;
    if before.command_id != command_id {
        crate::network::push_log(format!(
            "Action 观察器命令不匹配：开始={}，结束={}",
            before.command_id, command_id
        ));
        return None;
    }

    let after_without_chunks = capture(command_id, false);
    let before_guids: HashSet<_> = before
        .tracks
        .iter()
        .map(|track| track.guid.clone())
        .collect();
    let after_guids: HashSet<_> = after_without_chunks
        .tracks
        .iter()
        .map(|track| track.guid.clone())
        .collect();
    let added_guids: HashSet<_> = after_guids.difference(&before_guids).cloned().collect();
    let after = if added_guids.is_empty() {
        after_without_chunks
    } else {
        capture(command_id, true)
    };
    let added_tracks = after
        .tracks
        .iter()
        .filter(|track| added_guids.contains(&track.guid))
        .cloned()
        .collect();
    let removed_track_guids = before
        .tracks
        .iter()
        .filter(|track| !after_guids.contains(&track.guid))
        .map(|track| track.guid.clone())
        .collect();
    let before_order: Vec<_> = before
        .tracks
        .iter()
        .map(|track| track.guid.clone())
        .collect();
    let after_order: Vec<_> = after
        .tracks
        .iter()
        .map(|track| track.guid.clone())
        .collect();

    Some(ActionChange {
        command_id,
        before,
        after,
        added_tracks,
        removed_track_guids,
        track_order_changed: before_order != after_order,
    })
}
