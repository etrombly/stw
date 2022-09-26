//#![warn(missing_docs)]
//! Wrapper library for SyncThing for remote development.
//! Configures local and remote SyncThing instances to connect over ssh tunnels.
use once_cell::sync::OnceCell;
use ssh2::Channel;
use std::sync::Mutex;

pub static mut CHANNEL: OnceCell<Mutex<Channel>> = OnceCell::new();

/// Config file management
pub mod config;
/// Signal handling
pub mod signal;
/// ssh related functions
pub mod ssh;
/// SyncThing related functions
pub mod st;
