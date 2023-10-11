use super::{
    data::MsgItem,
    session,
};
use crate::slint_generatedAppWindow::{AppWindow, ChatItem, ChatSession, Logic, Store};
use crate::util::translator::tr;
use crate::{config, util};
use base64;
use chrono::Utc;
use native_dialog::FileDialog;
use slint::{ComponentHandle, Model, VecModel, Weak};
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;
use uuid::Uuid;

const TEXT_TIMEOUT: i64 = 300;

pub fn init(ui: &AppWindow, tx: mpsc::UnboundedSender<String>) {
    let (ui_handle, tx_handle) = (ui.as_weak(), tx.clone());
    ui.global::<Logic>().on_send_text(move |text| {
        if text.trim().is_empty() {
            return;
        }

        let ui = ui_handle.unwrap();
        let suuid = ui.global::<Store>().get_current_session_uuid();

        for (index, mut session) in ui.global::<Store>().get_chat_sessions().iter().enumerate() {
            if session.uuid == suuid {
                add_chat_timestamp(&mut session);
                ui.global::<Store>()
                    .get_chat_sessions()
                    .set_row_data(index, session);
                break;
            }
        }

        ui.global::<Store>()
            .get_session_datas()
            .as_any()
            .downcast_ref::<VecModel<ChatItem>>()
            .expect("We know we set a VecModel earlier")
            .push(ChatItem {
                r#type: "uitem".into(),
                text: text.clone(),
                ..Default::default()
            });

        let mut mi = MsgItem::default();
        mi.r#type = "plain".to_string();
        mi.to_uuid = suuid.to_string();
        mi.text = text.to_string();
        send_msg(&ui, tx_handle.clone(), mi);
    });

    let (ui_handle, tx_handle) = (ui.as_weak(), tx.clone());
    ui.global::<Logic>().on_send_image(move || {
        let ui = ui_handle.unwrap();

        let image_path = match FileDialog::new()
            .set_location("~")
            .add_filter("Image Files", &["png", "PNG"])
            .show_open_single_file()
        {
            Ok(Some(file)) => file,
            Err(e) => {
                ui.global::<Logic>().invoke_show_message(
                    slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                    "warning".into(),
                );

                return;
            }
            _ => return,
        };

        send_image(&ui, tx_handle.clone(), &image_path);
    });

    let (ui_handle, tx_handle) = (ui.as_weak(), tx.clone());
    ui.global::<Logic>().on_retry_send_text(move || {
        let ui = ui_handle.unwrap();
        let rows = ui.global::<Store>().get_session_datas().row_count();
        if rows == 0 {
            return;
        }

        let item = ui
            .global::<Store>()
            .get_session_datas()
            .as_any()
            .downcast_ref::<VecModel<ChatItem>>()
            .expect("We know we set a VecModel earlier")
            .row_data(rows - 1)
            .unwrap();

        match item.r#type.as_str() {
            "uitem" | "uimage" => {
                let item = ui
                    .global::<Store>()
                    .get_session_datas()
                    .as_any()
                    .downcast_ref::<VecModel<ChatItem>>()
                    .expect("We know we set a VecModel earlier")
                    .remove(rows - 1);

                if item.r#type.as_str() == "uitem" {
                    ui.global::<Logic>().invoke_send_text(item.text);
                } else if item.r#type.as_str() == "uimage" {
                    let image_path = Path::new(item.img_path.as_str());
                    send_image(&ui, tx_handle.clone(), &image_path);
                }

                ui.global::<Logic>()
                    .invoke_show_message(tr("正在重试...").into(), "success".into());
            }
            _ => return,
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_save_image(move |image_path| {
        let ui = ui_handle.unwrap();
        let dst_file = match FileDialog::new()
            .set_location("~")
            .add_filter("Image Files", &["png", "PNG"])
            .show_open_single_file()
        {
            Ok(Some(file)) => file,
            Err(e) => {
                ui.global::<Logic>().invoke_show_message(
                    slint::format!("{}. {}: {:?}", tr("保存失败"), tr("原因"), e),
                    "warning".into(),
                );
                return;
            }
            _ => return,
        };

        let src_file = Path::new(image_path.as_str());

        match fs::copy(src_file, dst_file) {
            Err(e) => {
                ui.global::<Logic>().invoke_show_message(
                    slint::format!("{}. {}: {:?}", tr("保存失败"), tr("原因"), e),
                    "warning".into(),
                );
            }
            _ => {
                ui.global::<Logic>()
                    .invoke_show_message(tr("保存成功").into(), "success".into());
            }
        }
    });
}

fn send_msg(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, item: MsgItem) {
    match serde_json::to_string(&item) {
        Ok(text) => {
            if let Err(e) = tx.send(text) {
                ui.global::<Logic>().invoke_show_message(
                    slint::format!("{}. {}: {}", tr("发送失败"), tr("原因"), e),
                    "warning".into(),
                );
            }
        }
        Err(e) => {
            ui.global::<Logic>().invoke_show_message(
                slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                "warning".into(),
            );
        }
    };
}

pub fn recv_cb(
    ui: Weak<AppWindow>,
    tx: mpsc::UnboundedSender<String>,
    msg: String,
    local_peer_id: String,
) {
    let ui = ui.unwrap();
    let sitem = MsgItem::from(msg.as_str());

    match sitem.r#type.as_str() {
        "handshake-req" => {
            if local_peer_id != sitem.text {
                return;
            }
            handle_handshake_request(&ui, tx.clone(), sitem);
        }
        "flush-req" => handle_flush_request(&ui, tx.clone(), sitem),
        _ => {
            if sitem.to_uuid != config::app_uuid() {
                return;
            }
            match sitem.r#type.as_str() {
                "handshake-res" => handle_handshake_respond(&ui, sitem),
                "flush-res" => handle_flush_respond(&ui, sitem),
                "plain" | "image" => handle_msg(&ui, sitem),
                _ => (),
            }
        }
    }
}

fn handle_msg(ui: &AppWindow, sitem: MsgItem) {
    let mut is_exist = false;
    for session in ui.global::<Store>().get_chat_sessions().iter() {
        if session.uuid.as_str() == sitem.from_uuid.as_str() {
            is_exist = true;
            break;
        }
    }

    if !is_exist {
        session::add_session(ui, &sitem);
    }

    let cur_suuid = ui.global::<Store>().get_current_session_uuid();
    for (index, mut session) in ui.global::<Store>().get_chat_sessions().iter().enumerate() {
        if session.uuid.as_str() == sitem.from_uuid.as_str() {
            add_chat_timestamp(&mut session);

            match sitem.r#type.as_str() {
                "plain" => add_chat_text(&session, &sitem),
                "image" => add_chat_image(&ui, &session, &sitem),
                _ => (),
            }

            session.status = sitem.status.into();
            session.unread_count = if session.uuid != cur_suuid {
                session.unread_count + 1
            } else {
                0
            };

            ui.global::<Store>()
                .get_chat_sessions()
                .set_row_data(index, session);

            return;
        }
    }
}

fn add_chat_timestamp(session: &mut ChatSession) {
    let ts = Utc::now().timestamp();
    if session.chat_items.row_count() == 0_usize
        || ts - session.timestamp.parse::<i64>().unwrap_or(0_i64) > TEXT_TIMEOUT
    {
        session
            .chat_items
            .as_any()
            .downcast_ref::<VecModel<ChatItem>>()
            .expect("We know we set a VecModel earlier")
            .push(ChatItem {
                r#type: "timestamp".into(),
                text: util::time::local_now("%m-%d %H:%M:%S").into(),
                ..Default::default()
            });
    }

    session.timestamp = slint::format!("{ts}");
}

#[allow(dead_code)]
fn get_session(ui: &AppWindow, suuid: &slint::SharedString) -> Option<ChatSession> {
    for session in ui.global::<Store>().get_chat_sessions().iter() {
        if session.uuid == suuid {
            return Some(session);
        }
    }

    None
}

pub fn send_handshake_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, peer_id: String) {
    let mut mi = MsgItem::default();
    mi.r#type = "handshake-req".to_string();
    mi.text = peer_id.to_string();
    send_msg(ui, tx, mi);
}

pub fn send_flush_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>) {
    let mut mi = MsgItem::default();
    mi.r#type = "flush-req".to_string();
    send_msg(ui, tx, mi);
}

fn handle_flush_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, sitem: MsgItem) {
    let mut mi = MsgItem::default();
    mi.r#type = "flush-res".to_string();
    mi.to_uuid = sitem.from_uuid;
    send_msg(ui, tx, mi);
}

fn handle_flush_respond(ui: &AppWindow, sitem: MsgItem) {
    session::add_session(ui, &sitem);
}

fn handle_handshake_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, sitem: MsgItem) {
    session::add_session(ui, &sitem);

    let mut mi = MsgItem::default();
    mi.r#type = "handshake-res".to_string();
    mi.to_uuid = sitem.from_uuid;
    send_msg(ui, tx, mi);
}

fn handle_handshake_respond(ui: &AppWindow, sitem: MsgItem) {
    session::add_session(ui, &sitem);
}

fn add_chat_text(session: &ChatSession, sitem: &MsgItem) {
    session
        .chat_items
        .as_any()
        .downcast_ref::<VecModel<ChatItem>>()
        .expect("We know we set a VecModel earlier")
        .push(ChatItem {
            r#type: "bitem".into(),
            text: sitem.text.as_str().into(),
            ..Default::default()
        });
}

fn add_chat_image(ui: &AppWindow, session: &ChatSession, sitem: &MsgItem) {
    match base64::decode(&sitem.text) {
        Ok(data) => {
            let name = Uuid::new_v4().to_string();
            let img_path = Path::new(&config::cache_dir()).join(name.as_str());

            match fs::write(&img_path, &data) {
                Err(e) => {
                    ui.global::<Logic>().invoke_show_message(
                        slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                        "warning".into(),
                    );
                }
                _ => {
                    if let Ok(img) = slint::Image::load_from_path(&img_path) {
                        session
                            .chat_items
                            .as_any()
                            .downcast_ref::<VecModel<ChatItem>>()
                            .expect("We know we set a VecModel earlier")
                            .push(ChatItem {
                                r#type: "bimage".into(),
                                img,
                                img_path: img_path.to_str().unwrap().into(),
                                ..Default::default()
                            });
                    }
                }
            }
        }
        Err(e) => {
            ui.global::<Logic>().invoke_show_message(
                slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                "warning".into(),
            );
        }
    }
}

// TODO: split image to small chunk
fn send_image(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, image_path: &Path) {
    match slint::Image::load_from_path(&image_path) {
        Ok(img) => {
            let suuid = ui.global::<Store>().get_current_session_uuid();
            for (index, mut session) in ui.global::<Store>().get_chat_sessions().iter().enumerate()
            {
                if session.uuid == suuid {
                    add_chat_timestamp(&mut session);
                    ui.global::<Store>()
                        .get_chat_sessions()
                        .set_row_data(index, session);
                    break;
                }
            }

            ui.global::<Store>()
                .get_session_datas()
                .as_any()
                .downcast_ref::<VecModel<ChatItem>>()
                .expect("We know we set a VecModel earlier")
                .push(ChatItem {
                    r#type: "uimage".into(),
                    img,
                    img_path: image_path.to_str().unwrap().into(),
                    ..Default::default()
                });

            match fs::read(&image_path) {
                Ok(buffer) => {
                    let mut mi = MsgItem::default();
                    mi.r#type = "image".to_string();
                    mi.to_uuid = suuid.to_string();
                    mi.text = base64::encode(&buffer);
                    send_msg(ui, tx, mi);
                }
                Err(e) => {
                    ui.global::<Logic>().invoke_show_message(
                        slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                        "warning".into(),
                    );
                }
            }
        }
        Err(e) => {
            ui.global::<Logic>().invoke_show_message(
                slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                "warning".into(),
            );
        }
    }
}
