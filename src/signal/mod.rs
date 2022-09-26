#[cfg(target_os = "windows")]
use wintrap::trap;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use signal_hook::{consts::SIGINT, iterator::Signals, low_level};

use crate::CHANNEL;
use std::thread;
use std::io::Write;

#[cfg(target_os = "windows")]
pub fn init() {
    wintrap::trap(&[wintrap::Signal::CtrlC, wintrap::Signal::CtrlBreak, wintrap::Signal::CloseWindow], |signal| {}, move|| {
        unsafe {
            println!("Received signal {:?}", sig);
            if let Some(channel) = CHANNEL.get() {
                let mut channel = channel.lock().unwrap();
                let mut stream = channel.stream(0);
                let ctrl_c = format!("{}", 3 as char);
                while stream.write(ctrl_c.as_bytes()).is_err() {}
                while channel.send_eof().is_err() {}
                while channel.close().is_err() {}
            }
        }
    });
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn init() {
    let mut signals = Signals::new(&[SIGINT]).unwrap();

    thread::spawn(move || {
        for sig in signals.forever() {
            unsafe {
                println!("Received signal {:?}", sig);
                if let Some(channel) = CHANNEL.get() {
                    let mut channel = channel.lock().unwrap();
                    let mut stream = channel.stream(0);
                    let ctrl_c = format!("{}", 3 as char);
                    while stream.write(ctrl_c.as_bytes()).is_err() {}
                    while channel.send_eof().is_err() {}
                    while channel.close().is_err() {}
                }
                low_level::emulate_default_handler(SIGINT).unwrap();
            }
        }
    });
}
