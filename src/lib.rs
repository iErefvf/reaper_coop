mod network;
mod protocol;

use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use reaper_macros::reaper_extension_plugin;
use reaper_high::{ActionKind, Reaper};

use crate::network::client::join_room;
use crate::network::discovery::{local_ip, start_broadcast, start_discovery, RoomInfo};
use crate::network::server::{start_server, MemberTable};

#[reaper_extension_plugin(
    name = "reaper_coop",
    support_email_address = "",
    update_url = ""
)]
fn plugin_main() -> Result<(), Box<dyn Error>> {
    let reaper = Reaper::get();
    reaper.wake_up()?;

    let broadcast_running = Arc::new(AtomicBool::new(false));

    // ---------- Action 1: 创建房间（UDP 广播 + TCP Server） ----------
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

            // 启动 UDP 广播
            let info = RoomInfo {
                room_id: "room-001".to_string(),
                room_name: "我的房间".to_string(),
                room_type: 1,
                member_count: 1,
                host_ip: local_ip(),
            };
            start_broadcast(info, br.clone());

            // 启动 TCP Server
            let members: MemberTable = Arc::new(Mutex::new(HashMap::new()));
            let next_id = Arc::new(AtomicU64::new(1));
            start_server(members, next_id);

            Reaper::get().show_console_msg("房间已创建（UDP 广播 + TCP Server）\n");
        },
        ActionKind::NotToggleable,
    );
    std::mem::forget(action1);

    // ---------- Action 2: 搜索房间（一次性） ----------
    let action2 = reaper.register_action(
        "coop_search_rooms",
        "协作: 搜索房间",
        None,
        || {
            let running = Arc::new(AtomicBool::new(true));
            let rooms: Arc<Mutex<HashMap<String, RoomInfo>>> =
                Arc::new(Mutex::new(HashMap::new()));

            start_discovery(rooms.clone(), running.clone());
            std::thread::sleep(std::time::Duration::from_secs(3));
            running.store(false, Ordering::Relaxed);

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

    // ---------- Action 3: 加入房间 ----------
    // 保存与房主的连接，让后续操作能复用它
    let client_conn: Arc<Mutex<Option<crate::network::client::ClientConnection>>> =
        Arc::new(Mutex::new(None));
    let conn_clone = client_conn.clone();
    let action3 = reaper.register_action(
        "coop_join_room",
        "协作: 加入房间",
        None,
        move || {
            let reaper = Reaper::get();
            let host_ip = match reaper.medium_reaper().get_user_inputs(
                "加入协作房间",
                1,
                "房主 IP",
                "127.0.0.1",
                256,
            ) {
                Some(inputs) => inputs.to_str().trim().to_owned(),
                None => {
                    reaper.show_console_msg("已取消加入房间\n");
                    return;
                }
            };

            if host_ip.is_empty() {
                reaper.show_console_msg("房主 IP 不能为空\n");
                return;
            }

            let name = "成员A";

            match join_room(&host_ip, name) {
                Ok(conn) => {
                    Reaper::get().show_console_msg(format!(
                        "加入成功，成员 ID={}\n",
                        conn.member_id
                    ));
                    *conn_clone.lock().unwrap() = Some(conn);
                }
                Err(e) => {
                    Reaper::get().show_console_msg(format!("加入失败: {}\n", e));
                }
            }
        },
        ActionKind::NotToggleable,
    );
    std::mem::forget(action3);

    // ---------- Action 4: 显示日志 ----------
    let action4 = reaper.register_action(
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
    std::mem::forget(action4);

    reaper.show_console_msg("reaper_coop 初始化完成\n");
    Ok(())
}