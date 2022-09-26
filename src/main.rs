#[cfg(any(target_os = "linux", target_os = "macos"))]
use signal_hook::{consts::SIGINT, iterator::Signals, low_level};

use std::{io::Write, process::exit, thread};

use stw::{config::load_config, CHANNEL};

fn main() {
    #[cfg(target_os = "windows")]
    wintrap::trap(
        &[
            wintrap::Signal::CtrlC,
            wintrap::Signal::CtrlBreak,
            wintrap::Signal::CloseWindow,
        ],
        |_| {
            unsafe {
                if let Some(channel) = CHANNEL.get() {
                    let mut channel = channel.lock().unwrap();
                    let mut stream = channel.stream(0);
                    let ctrl_c = format!("{}", 3 as char);
                    while stream.write(ctrl_c.as_bytes()).is_err() {}
                    while channel.send_eof().is_err() {}
                    while channel.close().is_err() {}
                }
            }
            exit(0);
        },
        move || {
            let mut config = load_config(None).unwrap();
            config.generate_config_templates().unwrap();
        },
    )
    .unwrap();

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        init();
        let mut config = load_config(None).unwrap();
        config.generate_config_templates().unwrap();
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn init() {
    let mut signals = Signals::new([SIGINT]).unwrap();

    thread::spawn(move || {
        for _sig in signals.forever() {
            unsafe {
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
