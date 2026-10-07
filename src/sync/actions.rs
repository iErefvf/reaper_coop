#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionCategory {
    TrackAdd,
    TrackDelete,
    TrackMove,
    TrackDuplicate,
    TrackTemplate,
    MediaItem,
    Fx,
    Project,
    Unknown,
}

#[derive(Clone, Copy, Debug)]
pub struct ActionDefinition {
    pub id: u32,
    pub name: &'static str,
    pub category: ActionCategory,
    pub versions: &'static [&'static str],
}

// REAPER 版本新增或变更 Action ID 时，只需在这里维护。
// 未登记的 Action 仍由 observer 进行通用前后状态检测。
pub const ACTIONS: &[ActionDefinition] = &[
    ActionDefinition {
        id: 40001,
        name: "Track: Insert new track",
        category: ActionCategory::TrackAdd,
        versions: &["7.41", "7.81", "7.82"],
    },
    ActionDefinition {
        id: 40204,
        name: "Track: Insert new track",
        category: ActionCategory::TrackAdd,
        versions: &["7.41"],
    },
    ActionDefinition {
        id: 40906,
        name: "Track: Insert new track at end of track list",
        category: ActionCategory::TrackAdd,
        versions: &["7.81", "7.82"],
    },
    ActionDefinition {
        id: 40005,
        name: "Track: Insert multiple new tracks",
        category: ActionCategory::TrackAdd,
        versions: &["7.41"],
    },
    ActionDefinition {
        id: 40062,
        name: "Track: Insert multiple new tracks",
        category: ActionCategory::TrackAdd,
        versions: &["7.81", "7.82"],
    },
    ActionDefinition {
        id: 40702,
        name: "Track: Insert multiple new tracks",
        category: ActionCategory::TrackAdd,
        versions: &["7.82"],
    },
    ActionDefinition {
        id: 40364,
        name: "Track: Insert track from template",
        category: ActionCategory::TrackTemplate,
        versions: &["7.41", "7.81", "7.82"],
    },
    ActionDefinition {
        id: 42398,
        name: "Track: Insert multiple tracks from template",
        category: ActionCategory::TrackTemplate,
        versions: &["7.82"],
    },
];

pub fn find(command_id: u32) -> Option<&'static ActionDefinition> {
    ACTIONS.iter().find(|action| action.id == command_id)
}
