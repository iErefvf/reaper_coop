use crate::protocol::{OpKind, Operation};

pub fn apply_op_locally(op: &Operation) {
    match &op.kind {
        OpKind::TrackAdd { index } => {
            apply_track_add(*index);
        }
        OpKind::TrackDelete { track_guid } => {
            crate::network::push_log(format!("暂未实现 TrackDelete: {}", track_guid));
        }
    }
}

fn apply_track_add(index: i32) {
    let reaper = reaper_high::Reaper::get();
    let project = reaper.current_project();
    let count = project.track_count();
    let idx = if index < 0 {
        count
    } else {
        index as u32
    };

    match project.insert_track_at(idx) {
        Ok(_) => crate::network::push_log(format!("已插入轨道，索引={}", idx)),
        Err(err) => crate::network::push_log(format!("插入轨道失败: {}", err)),
    }
}