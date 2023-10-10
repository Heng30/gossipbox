use crate::{config, util};
use log::{info, warn};
use tokio::{
    sync::mpsc,
    task,
    time::{sleep, Duration},
};

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
            let text = format!("ping-{}", util::time::timestamp_millisecond());
            if let Err(e) = tx.clone().send(text) {
                warn!("{e:?}");
            }
            sleep(Duration::from_secs(swarm_conf.ping_interval)).await;
        }
    });
}
