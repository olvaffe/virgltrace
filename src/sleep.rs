extern crate libc;

use std::sync::{Arc, Condvar, Mutex};
use std::time;

static mut SIGNAL: Option<Arc<(Mutex<()>, Condvar)>> = None;

unsafe extern "C" fn signal_handler(_: libc::c_int) {
    let signal = SIGNAL.as_ref().unwrap();
    let _guard = signal.0.lock().unwrap();
    signal.1.notify_one();
}

pub fn sleep(secs : u32) {
    let dur = time::Duration::from_secs(secs.into());

    let signal = Arc::new((Mutex::new(()), Condvar::new()));
    let guard = signal.0.lock().unwrap();

    unsafe {
        SIGNAL = Some(signal.clone());
        libc::signal(libc::SIGINT, signal_handler as usize);
    }

    let _ = signal.1.wait_timeout(guard, dur);

    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_DFL);
        SIGNAL = None;
    }
}
