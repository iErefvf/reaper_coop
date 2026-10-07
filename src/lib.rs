mod network;
mod protocol;
mod sync;

use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use reaper_macros::reaper_extension_plugin;
use reaper_high::{ActionKind, Reaper};

use crate::network::client::join_room;
use crate::network::discovery::{local_ip, start_broadcast, start_discovery, RoomInfo};
use crate::network::server::{host_submit_op, start_server, GlobalLog, MemberTable};
use crate::sync::hook::{CoopHook, CoopPostHook};

extern "C" fn coop_sync_timer() {
    let ops = crate::sync::queue::drain();
    for op in ops {
        crate::sync::apply::apply_op_locally(&op);
    }
}

#[reaper_extension_plugin(
    name = "reaper_coop",
    support_email_address = "",
    update_url = ""
)]
fn plugin_main() -> Result<(), Box<dyn Error>> {
    let reaper = Reaper::get();
    reaper.wake_up()?;

    if let Err(error) = crate::network::initialize() {
        reaper.show_console_msg(format!(
            "协作网络初始化失败：{}\n请检查端口占用、防火墙和网络权限。\n",
            error
        ));
    } else {
        reaper.show_console_msg("协作网络初始化完成\n");
    }

    // ---------- 注册普通 Action hook ----------
    {
        let mut session = reaper.medium_session();
        session
            .plugin_register_add_hook_command::<CoopHook>()
            .map_err(|e| format!("注册 hookcommand 失败: {:?}", e))?;
        session
            .plugin_register_add_hook_post_command::<CoopPostHook>()
            .map_err(|e| format!("注册 hookpostcommand 失败: {:?}", e))?;
        session
            .plugin_register_add_timer(coop_sync_timer)
            .map_err(|e| format!("注册同步定时器失败: {:?}", e))?;
    }
    reaper.show_console_msg("hookcommand 已注册\n");
    reaper.show_console_msg("同步定时器已注册\n");

    let broadcast_running = Arc::new(AtomicBool::new(false));

    // ---------- Action 1: 创建房间 ----------
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

            let info = RoomInfo {
                room_id: "room-001".to_string(),
                room_name: "我的房间".to_string(),
                room_type: 1,
                member_count: 1,
                host_ip: local_ip(),
            };

            let log = Arc::new(Mutex::new(GlobalLog::new()));
            let members: MemberTable = Arc::new(Mutex::new(HashMap::new()));
            let next_id = Arc::new(AtomicU64::new(1));

            if let Err(error) = start_server(members.clone(), next_id, log.clone()) {
                Reaper::get().show_console_msg(format!("房间创建失败: {}\n", error));
                return;
            }

            br.store(true, Ordering::Relaxed);
            start_broadcast(info, br.clone());

            // 设置房主模式的 OP_SENDER
            let members_for_sender = members.clone();
            let log_for_sender = log.clone();
            crate::sync::set_op_sender(Box::new(move |op| {
                host_submit_op(op, &members_for_sender, &log_for_sender);
            }));

            Reaper::get().show_console_msg("房间已创建\n");
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

                    // 设置成员模式的 OP_SENDER：通过 TCP 发给房主
                    let send_stream = match conn.stream.try_clone() {
                        Ok(stream) => stream,
                        Err(error) => {
                            Reaper::get().show_console_msg(format!(
                                "加入失败：复制连接失败: {}\n",
                                error
                            ));
                            return;
                        }
                    };
                    crate::sync::set_op_sender(Box::new(move |op| {
                        use std::io::Write;
                        let msg = crate::protocol::Message::OpRequest { op };
                        let data = crate::protocol::encode_message(&msg);
                        match send_stream.try_clone() {
                            Ok(mut stream) => {
                                if let Err(error) = stream.write_all(&data) {
                                    crate::network::push_log(format!(
                                        "发送操作失败: {}",
                                        error
                                    ));
                                }
                            }
                            Err(error) => crate::network::push_log(format!(
                                "复制发送连接失败: {}",
                                error
                            )),
                        }
                    }));

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

    // ---------- Action 5: 测试插入轨道（临时，验证同步链路用） ----------
    let action5 = reaper.register_action(
        "coop_test_add_track",
        "协作: 测试插入轨道",
        None,
        || {
            // 直接走 send_op，等价于 hook 触发后的流程
            let op = crate::protocol::Operation {
                id: 0,
                seq: 0,
                author: 0,
                applied_locally: false,
                kind: crate::protocol::OpKind::TrackAdd {
                    index: -1,
                    chunks: vec![],
                },
                undo_data: vec![],
            };
            crate::sync::send_op(op);
            Reaper::get().show_console_msg("已发送测试插入轨道操作\n");
        },
        ActionKind::NotToggleable,
    );
    std::mem::forget(action5);

    reaper.show_console_msg("reaper_coop 初始化完成\n");
    Ok(())
}