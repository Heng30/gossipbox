use chrono::Local;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn local_now(format: &str) -> String {
    return Local::now().format(format).to_string();
}

pub fn timestamp_millisecond() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since_the_epoch) => since_the_epoch.as_millis(),
        _ => 0,
    }
}
