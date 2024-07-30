use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_timestamp_in_secs() -> u64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).expect("Could not ").as_secs()
}
