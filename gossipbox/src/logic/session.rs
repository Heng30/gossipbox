use super::chat;
use super::data::SendItem;
use crate::slint_generatedAppWindow::{AppWindow, ChatItem, ChatSession, Logic, Store};
use crate::{config, util::translator::tr};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use tokio::sync::mpsc;

pub fn init(ui: &AppWindow, tx: mpsc::UnboundedSender<String>) {
    ui.global::<Store>().set_user_name(config::name().into());

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_reset_current_session_chats(move || {
            let ui = ui_handle.unwrap();
            ui.global::<Store>()
                .get_session_datas()
                .as_any()
                .downcast_ref::<VecModel<ChatItem>>()
                .expect("We know we set a VecModel earlier")
                .set_vec(vec![]);

            ui.global::<Logic>()
                .invoke_show_message(tr("清空成功").into(), "success".into());
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_switch_session(move |_old_uuid, new_uuid| {
            let ui = ui_handle.unwrap();
            for session in ui.global::<Store>().get_chat_sessions().iter() {
                if session.uuid == new_uuid {
                    ui.global::<Store>()
                        .set_session_datas(session.chat_items.clone());

                    ui.global::<Store>().set_current_session_uuid(new_uuid);
                    return;
                }
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_flush_sessions(move || {
        let ui = ui_handle.unwrap();
        ui.global::<Logic>()
            .invoke_show_message(tr("刷新...").into(), "info".into());

        for (index, mut session) in ui.global::<Store>().get_chat_sessions().iter().enumerate() {
            let uuid = session.uuid.to_string();
            session.status = "offline".into();
            ui.global::<Store>()
                .get_chat_sessions()
                .set_row_data(index, session);

            chat::send_flush_request(&ui, tx.clone(), uuid);
        }

        ui.global::<Logic>()
            .invoke_show_message(tr("刷新成功").into(), "success".into());
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_set_user_name(move |name| {
        let ui = ui_handle.unwrap();
        ui.global::<Store>().set_user_name(name.clone());

        match config::set_name(name.to_string()) {
            Err(e) => ui.global::<Logic>().invoke_show_message(
                slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                "warning".into(),
            ),
            _ => (),
        }
    });
}

pub fn add_session(ui: &AppWindow, sitem: SendItem) {
    for (index, mut session) in ui.global::<Store>().get_chat_sessions().iter().enumerate() {
        if session.uuid.as_str() == sitem.from_uuid.as_str() {
            session.name = sitem.name.into();
            session.status = "online".into();

            ui.global::<Store>()
                .get_chat_sessions()
                .set_row_data(index, session);

            return;
        }
    }

    let chat_items = ModelRc::new(VecModel::default());
    ui.global::<Store>()
        .get_chat_sessions()
        .as_any()
        .downcast_ref::<VecModel<ChatSession>>()
        .expect("We know we set a VecModel earlier")
        .push(ChatSession {
            uuid: sitem.from_uuid.as_str().into(),
            name: sitem.name.into(),
            status: "online".into(),
            chat_items: chat_items.clone(),
            ..Default::default()
        });

    if ui.global::<Store>().get_chat_sessions().row_count() == 1 {
        ui.global::<Store>().set_session_datas(chat_items);
        ui.global::<Store>()
            .set_current_session_uuid(sitem.from_uuid.as_str().into());
    }
}
