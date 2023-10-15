use super::data::{MsgItem, RecvFileCBArgs};
use crate::slint_generatedAppWindow::{AppWindow, Logic};
use crate::util::translator::tr;
use crate::{RecvFileCB, SendFileCB};
use log::{debug, info};
use slint::ComponentHandle;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::timeout;

fn show_error(ui: &AppWindow, estr: String) {
    let ui = ui.as_weak();
    let _ = slint::invoke_from_event_loop(move || {
        ui.unwrap().global::<Logic>().invoke_show_message(
            slint::format!("{}. {}: {:?}", tr("发送失败"), tr("原因"), estr),
            "warning".into(),
        );
    });
}

pub fn send(
    file_path: String,
    ui: &AppWindow,
    cb: SendFileCB,
    mi: MsgItem,
    tx: mpsc::UnboundedSender<String>,
) {
    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        match TcpListener::bind("0:0").await {
            Ok(listener) => {
                let listen_port = match listener.local_addr() {
                    Ok(addr) => {
                        info!("Listening at {addr:?}");
                        addr.port()
                    }
                    Err(e) => {
                        show_error(&ui_handle.clone().unwrap(), e.to_string());
                        return;
                    }
                };

                // Send msg to peer and tell it to download the file
                let ui = ui_handle.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    cb(ui, mi, listen_port, tx);
                });

                match timeout(Duration::from_secs(15), listener.accept()).await {
                    Ok(Ok((mut socket, _))) => {
                        debug!("Peer is connected. Sending file: {file_path}");
                        match File::open(file_path.to_owned()).await {
                            Ok(file) => {
                                match tokio::io::copy(&mut BufReader::new(file), &mut socket).await
                                {
                                    Err(e) => {
                                        show_error(&ui_handle.clone().unwrap(), e.to_string());
                                    }
                                    _ => (),
                                }
                            }
                            Err(e) => {
                                show_error(&ui_handle.clone().unwrap(), e.to_string());
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        show_error(&ui_handle.clone().unwrap(), e.to_string());
                    }
                    Err(_) => {
                        show_error(&ui_handle.clone().unwrap(), "Accept timed out".to_string());
                    }
                }
            }
            Err(e) => {
                show_error(&ui_handle.clone().unwrap(), e.to_string());
            }
        };
    });
}

pub fn recv(
    ui: &AppWindow,
    args: RecvFileCBArgs,
    cb: RecvFileCB,
    suuid: String,
    save_path: String,
) {
    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        let dfi = match args {
            RecvFileCBArgs::Image(ref item) => item.dfi.clone(),
            RecvFileCBArgs::File(ref item) => item.dfi.clone(),
        };

        for ip in dfi.ips.into_iter() {
            let addr = format!("{}:{}", ip, dfi.port);

            match timeout(Duration::from_secs(3), TcpStream::connect(addr)).await {
                Ok(Ok(mut stream)) => {
                    info!("Peer is connected. Receiving file: {save_path:?}");

                    match File::create(&save_path).await {
                        Ok(file) => {
                            match tokio::io::copy(&mut stream, &mut BufWriter::new(file)).await {
                                Err(e) => {
                                    show_error(&ui_handle.clone().unwrap(), e.to_string());
                                }
                                _ => {
                                    let ui = ui_handle.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        cb(ui, suuid, save_path, args);
                                    });
                                }
                            }
                            return;
                        }
                        Err(e) => {
                            show_error(&ui_handle.clone().unwrap(), e.to_string());
                        }
                    }
                }
                _ => (),
            }
        }
    });
}
