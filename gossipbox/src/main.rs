#![windows_subsystem = "windows"]

slint::include_modules!();

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

use slint::Weak;
use chrono::Local;
use env_logger::fmt::Color as LColor;
use log::debug;
use std::env;
use std::io::Write;
use tokio::sync::mpsc;

mod config;
mod logic;
mod util;
mod version;

use logic::{about, chat, clipboard, message, ok_cancel_dialog, session, setting, window};

pub type CResult = Result<(), Box<dyn std::error::Error>>;
pub type SendCB = fn (ui: Weak<AppWindow>, tx: mpsc::UnboundedSender<String>, msg: String, local_peer_id: String);

#[tokio::main]
async fn main() -> CResult {
    init_logger();
    debug!("{}", "start...");

    config::init();

    let ui = AppWindow::new().unwrap();
    logic::util::init(&ui);
    logic::base::init(&ui);

    let tx = util::svr::init(ui.as_weak(), chat::recv_cb);

    clipboard::init(&ui);
    message::init(&ui);
    session::init(&ui, tx.clone());
    chat::init(&ui, tx);
    window::init(&ui);
    about::init(&ui);
    setting::init(&ui);

    ok_cancel_dialog::init(&ui);
    ui.run().unwrap();

    debug!("{}", "exit...");
    Ok(())
}

fn init_logger() {
    env_logger::builder()
        .format(|buf, record| {
            let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
            let mut level_style = buf.style();
            match record.level() {
                log::Level::Warn | log::Level::Error => {
                    level_style.set_color(LColor::Red).set_bold(true)
                }
                _ => level_style.set_color(LColor::Blue).set_bold(true),
            };

            writeln!(
                buf,
                "[{} {} {} {}] {}",
                ts,
                level_style.value(record.level()),
                record
                    .file()
                    .unwrap_or("None")
                    .split('/')
                    .last()
                    .unwrap_or("None"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
}
