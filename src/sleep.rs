use std::{thread, time};

pub fn sleep(secs : u32) {
    thread::sleep(time::Duration::from_secs(secs.into()));
}
