use std::time::SystemTime;

pub fn get_time() -> f64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs_f64(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}