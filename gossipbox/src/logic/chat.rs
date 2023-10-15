use super::{
    data::{ChatFileArgs, ChatImgArgs, DynFileSvrInfo, FileInfo, MsgItem, RecvFileCBArgs},
    filesvr, session,
};
use crate::slint_generatedAppWindow::{AppWindow, ChatItem, ChatSession, Logic, Store};
use crate::util::translator::tr;
use crate::{config, util};
use chrono::Utc;
use log::info;
use native_dialog::FileDialog;
use slint::{ComponentHandle, Model, VecModel, Weak};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tokio::sync::mpsc;
use uuid::Uuid;

lazy_static! {
    pub static ref SEND_FILEINFO_CACHE: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

lazy_static! {
    pub static ref RECV_FILEINFO_CACHE: Mutex<HashMap<String, (String, String)>> =
        Mutex::new(HashMap::new());
}

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
                uuid: Uuid::new_v4().to_string().into(),
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
    ui.global::<Logic>().on_send_file(move || {
        let ui = ui_handle.unwrap();

        let file_path = match FileDialog::new().set_location("~").show_open_single_file() {
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

        send_fileinfo(&ui, tx_handle.clone(), file_path.as_path());
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
            "uitem" | "uimage" | "ufile" => {
                let item = ui
                    .global::<Store>()
                    .get_session_datas()
                    .as_any()
                    .downcast_ref::<VecModel<ChatItem>>()
                    .expect("We know we set a VecModel earlier")
                    .remove(rows - 1);

                match item.r#type.as_str() {
                    "uitem" => {
                        ui.global::<Logic>().invoke_send_text(item.text);
                    }
                    "uimage" => {
                        let image_path = Path::new(item.img_path.as_str());
                        send_image(&ui, tx_handle.clone(), &image_path);
                    }
                    "ufile" => {
                        let fi = {
                            SEND_FILEINFO_CACHE
                                .lock()
                                .unwrap()
                                .get(item.file_id.as_str())
                                .and_then(|item| Some(item.clone()))
                        };
                        match fi {
                            Some(file_path) => {
                                send_fileinfo(
                                    &ui,
                                    tx_handle.clone(),
                                    &Path::new(file_path.as_str()),
                                );
                            }
                            _ => {
                                ui.global::<Logic>().invoke_show_message(
                                    slint::format!(
                                        "{}. {}: {:?}",
                                        tr("出错"),
                                        tr("原因"),
                                        "file path not in cache"
                                    ),
                                    "warning".into(),
                                );
                            }
                        }
                    }
                    _ => (),
                }

                ui.global::<Logic>()
                    .invoke_show_message(tr("正在重试...").into(), "success".into());
            }
            _ => return,
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_chat_file_item(move |uuid| {
        let ui = ui_handle.unwrap();

        for (index, item) in ui.global::<Store>().get_session_datas().iter().enumerate() {
            if item.uuid == uuid {
                ui.global::<Store>()
                    .get_session_datas()
                    .as_any()
                    .downcast_ref::<VecModel<ChatItem>>()
                    .expect("We know we set a VecModel earlier")
                    .remove(index);

                ui.global::<Logic>()
                    .invoke_show_message(tr("删除成功").into(), "success".into());

                SEND_FILEINFO_CACHE
                    .lock()
                    .unwrap()
                    .remove(item.file_id.as_str());
                return;
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_save_image(move |image_path| {
        let ui = ui_handle.unwrap();

        let dst_file = match FileDialog::new()
            .set_location("~")
            .set_filename("tmp.png")
            .show_save_single_file()
        {
            Ok(Some(file)) => file,
            Err(e) => {
                ui.global::<Logic>().invoke_show_message(
                    slint::format!("{}{:?}", tr("保存失败"), e),
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

    let (ui_handle, tx_handle) = (ui.as_weak(), tx.clone());
    ui.global::<Logic>()
        .on_download_file(move |uuid, file_id, filename| {
            let ui = ui_handle.unwrap();
            let suuid = ui.global::<Store>().get_current_session_uuid();

            let dst_file = match FileDialog::new()
                .set_location("~")
                .set_filename(filename.as_str())
                .show_save_single_file()
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

            {
                RECV_FILEINFO_CACHE.lock().unwrap().insert(
                    file_id.to_string().clone(),
                    (
                        uuid.to_string(),
                        dst_file.to_str().unwrap_or("").to_string(),
                    ),
                );
            }

            let mut mi = MsgItem::default();
            mi.r#type = "download-req".to_string();
            mi.to_uuid = suuid.to_string();
            mi.text = file_id.to_string();
            send_msg(&ui, tx_handle.clone(), mi);
            update_file_status(&ui, suuid.as_str(), uuid.as_str(), "downloading");

            ui.global::<Logic>()
                .invoke_show_message(tr("正在下载...").into(), "success".into());
        });
}

fn update_file_status(ui: &AppWindow, suuid: &str, uuid: &str, status: &str) {
    for session in ui.global::<Store>().get_chat_sessions().iter() {
        if session.uuid.as_str() == suuid {
            for (index, mut item) in session.chat_items.iter().enumerate() {
                if item.uuid.as_str() == uuid {
                    item.file_status = status.into();
                    ui.global::<Store>()
                        .get_session_datas()
                        .as_any()
                        .downcast_ref::<VecModel<ChatItem>>()
                        .expect("We know we set a VecModel earlier")
                        .set_row_data(index, item);
                    return;
                }
            }
            return;
        }
    }
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
                "plain" | "image" | "fileinfo" | "download-req" | "download-res" => {
                    handle_msg(&ui, tx.clone(), sitem)
                }
                _ => (),
            }
        }
    }
}

fn handle_msg(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, sitem: MsgItem) {
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
                "image" => add_chat_image(&ui, session.uuid.to_string(), &sitem),
                "fileinfo" => add_chat_file(&mut session, &sitem),
                "download-req" => send_download_res(&ui, &sitem, tx.clone()),
                "download-res" => start_download_file(&ui, &session, &sitem),
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
                uuid: Uuid::new_v4().to_string().into(),
                text: util::time::local_now("%m-%d %H:%M:%S").into(),
                ..Default::default()
            });
    }

    session.timestamp = slint::format!("{ts}");
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
            uuid: Uuid::new_v4().to_string().into(),
            text: sitem.text.as_str().into(),
            ..Default::default()
        });
}

fn add_chat_image(ui: &AppWindow, suuid: String, sitem: &MsgItem) {
    let name = format!("{}.png", Uuid::new_v4().to_string());
    let img_path = Path::new(&config::cache_dir())
        .join(name.as_str())
        .to_str()
        .unwrap_or("")
        .to_string();

    let args = RecvFileCBArgs::Image(ChatImgArgs {
        dfi: DynFileSvrInfo::from(sitem.text.as_str()),
    });

    filesvr::recv(ui, args, recv_image_fileinfo, suuid, img_path);
}

fn add_chat_file(session: &mut ChatSession, sitem: &MsgItem) {
    let fi = FileInfo::from(sitem.text.as_str());

    session
        .chat_items
        .as_any()
        .downcast_ref::<VecModel<ChatItem>>()
        .expect("We know we set a VecModel earlier")
        .push(ChatItem {
            r#type: "bfile".into(),
            uuid: Uuid::new_v4().to_string().into(),
            file_id: fi.id.into(),
            file_name: fi.name.into(),
            file_size: util::fs::file_size_string(fi.total_size).into(),
            file_status: "undownload".into(),
            ..Default::default()
        });
}

fn send_download_res(ui: &AppWindow, sitem: &MsgItem, tx: mpsc::UnboundedSender<String>) {
    let fpath = {
        SEND_FILEINFO_CACHE
            .lock()
            .unwrap()
            .get(sitem.text.as_str())
            .unwrap_or(&String::default())
            .clone()
    };

    if fpath.is_empty() {
        info!("can not download file, because it has been cancel.");
        return;
    }

    info!("send file path: {fpath:?}");

    let mut mi = MsgItem::default();
    mi.r#type = "download-res".to_string();
    mi.to_uuid = sitem.from_uuid.clone();
    mi.pri_data = sitem.text.clone();

    filesvr::send(fpath, ui, send_dynsvrinfo, mi, tx.clone());
}

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
                    uuid: Uuid::new_v4().to_string().into(),
                    img,
                    img_path: image_path.to_str().unwrap().into(),
                    ..Default::default()
                });

            let mut mi = MsgItem::default();
            mi.r#type = "image".to_string();
            mi.to_uuid = suuid.to_string();
            filesvr::send(
                image_path.to_str().unwrap_or("").to_string(),
                ui,
                send_dynsvrinfo,
                mi,
                tx,
            );
        }
        Err(e) => {
            ui.global::<Logic>().invoke_show_message(
                slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                "warning".into(),
            );
        }
    }
}

fn start_download_file(ui: &AppWindow, session: &ChatSession, sitem: &MsgItem) {
    let (uuid, file_path) = {
        match RECV_FILEINFO_CACHE
            .lock()
            .unwrap()
            .get(sitem.pri_data.as_str())
        {
            Some((u, f)) => (u.clone(), f.clone()),
            None => {
                ui.global::<Logic>().invoke_show_message(
                    slint::format!(
                        "{}. {}: {:?}",
                        tr("下载失败"),
                        tr("原因"),
                        "file not in the cache"
                    ),
                    "warning".into(),
                );
                return;
            }
        }
    };

    let args = RecvFileCBArgs::File(ChatFileArgs {
        uuid,
        dfi: DynFileSvrInfo::from(sitem.text.as_str()),
    });

    filesvr::recv(
        ui,
        args,
        recv_file_fileinfo,
        session.uuid.to_string(),
        file_path,
    );
}

fn send_dynsvrinfo(
    ui: Weak<AppWindow>,
    mut mi: MsgItem,
    listen_port: u16,
    tx: mpsc::UnboundedSender<String>,
) {
    let ui = ui.unwrap();
    let dfi = DynFileSvrInfo {
        ips: util::net::ipv4_interfaces(),
        port: listen_port,
    };

    match serde_json::to_string(&dfi) {
        Ok(text) => {
            mi.text = text;
            send_msg(&ui, tx, mi);
        }
        Err(e) => {
            ui.global::<Logic>().invoke_show_message(
                slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                "warning".into(),
            );
        }
    };
}

fn recv_image_fileinfo(
    ui: Weak<AppWindow>,
    suuid: String,
    img_path: String,
    _args: RecvFileCBArgs,
) {
    let ui = ui.unwrap();
    for (index, session) in ui.global::<Store>().get_chat_sessions().iter().enumerate() {
        if session.uuid.as_str() == suuid.as_str() {
            match slint::Image::load_from_path(Path::new(&img_path)) {
                Ok(img) => {
                    session
                        .chat_items
                        .as_any()
                        .downcast_ref::<VecModel<ChatItem>>()
                        .expect("We know we set a VecModel earlier")
                        .push(ChatItem {
                            r#type: "bimage".into(),
                            uuid: Uuid::new_v4().to_string().into(),
                            img,
                            img_path: img_path.into(),
                            ..Default::default()
                        });

                    ui.global::<Store>()
                        .get_chat_sessions()
                        .set_row_data(index, session);
                }
                Err(e) => {
                    ui.global::<Logic>().invoke_show_message(
                        slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                        "warning".into(),
                    );
                }
            }
            return;
        }
    }
}

fn recv_file_fileinfo(
    ui: Weak<AppWindow>,
    suuid: String,
    _save_path: String,
    args: RecvFileCBArgs,
) {
    let ui = ui.unwrap();
    match args {
        RecvFileCBArgs::File(arg) => {
            for session in ui.global::<Store>().get_chat_sessions().iter() {
                if session.uuid.as_str() == suuid.as_str() {
                    for (index, mut item) in session.chat_items.iter().enumerate() {
                        if item.uuid.as_str() == arg.uuid.as_str() {
                            item.file_status = "downloaded".into();
                            session
                                .chat_items
                                .as_any()
                                .downcast_ref::<VecModel<ChatItem>>()
                                .expect("We know we set a VecModel earlier")
                                .set_row_data(index, item);

                            ui.global::<Logic>()
                                .invoke_show_message(tr("下载成功").into(), "success".into());

                            return;
                        }
                    }
                    return;
                }
            }
        }
        _ => (),
    }
}

fn get_fileinfo(file_path: &Path) -> FileInfo {
    let total_size = util::fs::file_size(file_path);
    let name = file_path
        .file_name()
        .unwrap_or(OsStr::new(""))
        .to_str()
        .unwrap_or("")
        .to_string();

    FileInfo {
        id: Uuid::new_v4().to_string(),
        name,
        total_size,
    }
}

fn send_fileinfo(ui: &AppWindow, tx: mpsc::UnboundedSender<String>, file_path: &Path) {
    let suuid = ui.global::<Store>().get_current_session_uuid();
    let fi = get_fileinfo(file_path);

    {
        SEND_FILEINFO_CACHE
            .lock()
            .unwrap()
            .insert(fi.id.clone(), file_path.to_str().unwrap_or("").to_string());
    }

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
            r#type: "ufile".into(),
            uuid: fi.id.as_str().into(),
            file_id: fi.id.as_str().into(),
            file_name: fi.name.as_str().into(),
            file_size: util::fs::file_size_string(fi.total_size).into(),
            ..Default::default()
        });

    let mut mi = MsgItem::default();
    mi.r#type = "fileinfo".to_string();
    mi.to_uuid = suuid.to_string();

    match serde_json::to_string(&fi) {
        Ok(text) => {
            mi.text = text.to_string();
            send_msg(&ui, tx, mi);
        }
        Err(e) => {
            ui.global::<Logic>().invoke_show_message(
                slint::format!("{}. {}: {:?}", tr("出错"), tr("原因"), e),
                "warning".into(),
            );
        }
    };
}
