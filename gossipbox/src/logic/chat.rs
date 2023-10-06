use super::{data::SendItem, session};
use crate::slint_generatedAppWindow::{AppWindow, ChatItem, Logic, Store};
use crate::util::translator::tr;
use crate::{config, util};
use slint::{ComponentHandle, Model, VecModel, Weak};
use tokio::sync::mpsc;

pub fn init(ui: &AppWindow, tx: mpsc::UnboundedSender<String>) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_send_text(move |text| {
        if text.trim().is_empty() {
            return;
        }

        let ui = ui_handle.unwrap();
        let suuid = ui.global::<Store>().get_current_session_uuid();

        ui.global::<Store>()
            .get_session_datas()
            .as_any()
            .downcast_ref::<VecModel<ChatItem>>()
            .expect("We know we set a VecModel earlier")
            .push(ChatItem {
                r#type: "uitem".into(),
                text: text.clone(),
                timestamp: util::time::local_now("%m-%d %H:%M:%S").into(),
                ..Default::default()
            });

        send_text(
            &ui,
            tx.clone(),
            SendItem {
                r#type: "plain".to_string(),
                from_uuid: config::app_uuid(),
                to_uuid: suuid.to_string(),
                name: config::name(),
                text: text.to_string(),
                timestamp: util::time::timestamp_millisecond(),
                ..Default::default()
            },
        );
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_retry_send_text(move || {
        let ui = ui_handle.unwrap();

        let rows = ui.global::<Store>().get_session_datas().row_count();
        if rows == 0 {
            return;
        }

        if ui
            .global::<Store>()
            .get_session_datas()
            .as_any()
            .downcast_ref::<VecModel<ChatItem>>()
            .expect("We know we set a VecModel earlier")
            .row_data(rows - 1)
            .unwrap()
            .r#type
            == "bitem"
        {
            return;
        }

        let item = ui
            .global::<Store>()
            .get_session_datas()
            .as_any()
            .downcast_ref::<VecModel<ChatItem>>()
            .expect("We know we set a VecModel earlier")
            .remove(rows - 1);

        ui.global::<Logic>().invoke_send_text(item.text);
        ui.global::<Logic>()
            .invoke_show_message(tr("正在重试...").into(), "success".into());
    });
}

fn send_text(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, item: SendItem) {
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
    let sitem = SendItem::from(msg.as_str());

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
                "plain" => handle_plain_text(&ui, msg),
                _ => (),
            }
        }
    }
}

fn handle_plain_text(ui: &AppWindow, msg: String) {
    let sitem = SendItem::from(msg.as_str());
    let cur_suuid = ui.global::<Store>().get_current_session_uuid();

    for (index, mut session) in ui.global::<Store>().get_chat_sessions().iter().enumerate() {
        if session.uuid.as_str() == sitem.from_uuid.as_str() {
            session
                .chat_items
                .as_any()
                .downcast_ref::<VecModel<ChatItem>>()
                .expect("We know we set a VecModel earlier")
                .push(ChatItem {
                    r#type: "bitem".into(),
                    text: sitem.text.into(),
                    timestamp: util::time::local_now("%m-%d %H:%M:%S").into(),
                });

            if session.uuid != cur_suuid {
                session.unread_count = session.unread_count + 1;

                ui.global::<Store>()
                    .get_chat_sessions()
                    .set_row_data(index, session);
            }

            return;
        }
    }
}

pub fn send_handshake_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, peer_id: String) {
    send_text(
        ui,
        tx,
        SendItem {
            r#type: "handshake-req".to_string(),
            from_uuid: config::app_uuid(),
            name: config::name(),
            text: peer_id.to_string(),
            timestamp: util::time::timestamp_millisecond(),
            ..Default::default()
        },
    );
}

pub fn send_flush_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>) {
    send_text(
        ui,
        tx,
        SendItem {
            r#type: "flush-req".to_string(),
            from_uuid: config::app_uuid(),
            name: config::name(),
            timestamp: util::time::timestamp_millisecond(),
            ..Default::default()
        },
    );
}

fn handle_flush_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, sitem: SendItem) {
    send_text(
        ui,
        tx,
        SendItem {
            r#type: "flush-res".to_string(),
            from_uuid: config::app_uuid(),
            to_uuid: sitem.from_uuid,
            name: config::name(),
            timestamp: util::time::timestamp_millisecond(),
            ..Default::default()
        },
    );
}

fn handle_flush_respond(ui: &AppWindow, sitem: SendItem) {
    session::add_session(ui, sitem);
}

fn handle_handshake_request(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, sitem: SendItem) {
    session::add_session(ui, sitem.clone());

    send_text(
        ui,
        tx,
        SendItem {
            r#type: "handshake-res".to_string(),
            from_uuid: config::app_uuid(),
            to_uuid: sitem.from_uuid,
            name: config::name(),
            timestamp: util::time::timestamp_millisecond(),
            ..Default::default()
        },
    );
}

fn handle_handshake_respond(ui: &AppWindow, sitem: SendItem) {
    session::add_session(ui, sitem);
}
