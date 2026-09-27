mod network;
mod protocol;

use std::error::Error;
use reaper_macros::reaper_extension_plugin;
use reaper_high::{ActionKind, Reaper};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::network::discovery::{local_ip, start_broadcast, start_discovery, RoomInfo};

#[reaper_extension_plugin(
    name = "reaper_coop",
    support_email_address = "",
    update_url = ""
)]

fn plugin_main() -> Result<(), Box<dyn Error>> {
    let reaper = Reaper::get();
    reaper.wake_up()?;

    // 广播和监听的存活标志
    let broadcast_running = Arc::new(AtomicBool::new(false));

    // ---------- Action 1: 创建房间（启动广播） ----------
    let br = broadcast_running.clone();
    let action1 = reaper.register_action(
        "coop_create_room",
        "协作: 创建房间",
        None,
        move || {
            if br.load(Ordering::Relaxed) {
                Reaper::get().show_console_msg("房间已经在广播中\n");
                return;
            }
            br.store(true, Ordering::Relaxed);
            let info = RoomInfo {
                room_id: "room-001".to_string(),
                room_name: "我的房间".to_string(),
                room_type: 1,
                member_count: 1,
                host_ip: local_ip(),
            };
            start_broadcast(info, br.clone());
            Reaper::get().show_console_msg("房间广播已启动\n");
        },
        ActionKind::NotToggleable,
    );
    std::mem::forget(action1);

    // ---------- Action 2: 搜索房间（启动监听） ----------
    let action2 = reaper.register_action(
        "coop_search_rooms",
        "协作: 搜索房间",
        None,
        || {
            // 本次搜索专用的状态
            let running = Arc::new(AtomicBool::new(true));
            let rooms: Arc<Mutex<HashMap<String, RoomInfo>>> =
                Arc::new(Mutex::new(HashMap::new()));

            // 启动监听线程
            start_discovery(rooms.clone(), running.clone());

            // 主线程等待 3 秒收集广播
            std::thread::sleep(std::time::Duration::from_secs(3));

            // 停止监听线程
            running.store(false, Ordering::Relaxed);

            // 打印本次发现的房间
            let found = rooms.lock().unwrap();
            if found.is_empty() {
                Reaper::get().show_console_msg("未发现任何房间\n");
            } else {
                Reaper::get().show_console_msg(format!("发现 {} 个房间:\n", found.len()));
                for info in found.values() {
                    Reaper::get().show_console_msg(format!(
                        "  - {} (host={}, 成员数={})\n",
                        info.room_name, info.host_ip, info.member_count
                    ));
                }
            }
        },
        ActionKind::NotToggleable,
    );
    std::mem::forget(action2);

    // ---------- Action 3: 显示日志 ----------
    let action3 = reaper.register_action(
        "coop_show_logs",
        "协作: 显示日志",
        None,
        || {
            let logs = crate::network::drain_logs();
            if logs.is_empty() {
                Reaper::get().show_console_msg("(无新日志)\n");
            } else {
                for line in logs {
                    Reaper::get().show_console_msg(format!("{}\n", line));
                }
            }
        },
        ActionKind::NotToggleable,
    );
    std::mem::forget(action3);

    reaper.show_console_msg("reaper_coop 初始化完成\n");
    Ok(())
}