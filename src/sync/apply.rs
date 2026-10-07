use crate::protocol::{OpKind, Operation};

pub fn apply_op_locally(op: &Operation) {
    match &op.kind {
        OpKind::TrackAdd { index, chunks } => {
            apply_track_add(*index, chunks);
        }
        OpKind::TrackDelete { track_guid } => {
            crate::network::push_log(format!("暂未实现 TrackDelete: {}", track_guid));
        }
    }
}

fn apply_track_add(index: i32, chunks: &[String]) {
    let reaper = reaper_high::Reaper::get();
    let project = reaper.current_project();
    let count = project.track_count();
    let idx = if index < 0 {
        count
    } else {
        index as u32
    };

    for (offset, chunk) in chunks.iter().enumerate() {
        let insert_index = idx + offset as u32;
        match project.insert_track_at(insert_index) {
            Ok(track) => {
                if !chunk.is_empty() {
                    if let Err(error) = track.set_chunk(
                        reaper_high::Chunk::new(chunk.clone()),
                    ) {
                        crate::network::push_log(format!(
                            "设置新增轨道状态失败，索引={}: {}",
                            insert_index, error
                        ));
                    }
                }
            }
            Err(err) => {
                crate::network::push_log(format!("插入轨道失败，索引={}: {}", insert_index, err));
                return;
            }
        }
    }
    if chunks.is_empty() {
        match project.insert_track_at(idx) {
            Ok(_) => {}
            Err(err) => {
                crate::network::push_log(format!("插入轨道失败，索引={}: {}", idx, err));
                return;
            }
        }
    }
    crate::network::push_log(format!(
        "已同步插入 {} 条轨道，起始索引={}",
        chunks.len().max(1),
        idx
    ));
}