use super::data::SendItem;
use crate::{config, util};
use log::{info, warn};
use chrono::Utc;
use tokio::{
    sync::mpsc,
    task,
    time::{sleep, Duration},
};
use std::sync::atomic::{AtomicI64, Ordering};

pub static LAST_PING_TIMESTAMP: AtomicI64 = AtomicI64::new(0);

pub fn set_timestimp(ts: i64) {
    LAST_PING_TIMESTAMP.store(ts, Ordering::SeqCst);
}

pub fn get_timestamp() -> i64 {
    LAST_PING_TIMESTAMP.load(Ordering::SeqCst)
}

pub fn init(tx: mpsc::UnboundedSender<String>) {
    if config::swarm().enable_ping {
        ping_timer(tx);
        info!("Enable ping timer");
    } else {
        info!("Disable ping timer");
    }
}

fn ping_timer(tx: mpsc::UnboundedSender<String>) {
    task::spawn(async move {
        let swarm_conf = config::swarm();
        loop {
            let ts = Utc::now().timestamp();
            if ts - get_timestamp() > (swarm_conf.ping_interval as i64) {
                if let Ok(text) = serde_json::to_string(&SendItem {
                    r#type: "ping".to_string(),
                    timestamp: util::time::timestamp_millisecond(),
                    ..Default::default()
                }) {
                    let (tx, text) = (tx.clone(), text.clone());
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Err(e) = tx.send(text) {
                            warn!("{e:?}");
                        }
                    });
                }

                set_timestimp(ts);
            }
            sleep(Duration::from_secs(1)).await;
        }
    });
}
