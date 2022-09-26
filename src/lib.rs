//#![warn(missing_docs)]
//! Wrapper library for SyncThing for remote development.
//! Configures local and remote SyncThing instances to connect over ssh tunnels.

/// Config file management
pub mod config;
/// ssh related functions
pub mod ssh;
/// SyncThing related functions
pub mod st;
