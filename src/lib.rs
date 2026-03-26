// SPDX-License-Identifier: MIT OR Apache-2.0

//! `scrcpy-protocol` — scrcpy protocol types and control primitives.
//!
//! This crate owns the wire protocol (video/audio/control packets),
//! H.264 stream parsing, and the [`ControlSender`] abstraction.
//! Higher-level concerns (server bootstrap, ADB tunnel, codec probing)
//! live in the `saide` application crate under `saide::scrcpy`.
//!
//! [scrcpy]: https://github.com/Genymobile/scrcpy

pub mod control_sender;
mod error;
pub mod h264;
pub mod protocol;

pub use crate::{
    control_sender::ControlSender,
    error::{IoError, Result, ScrcpyError},
};

pub const MAX_PACKET_SIZE: usize = 10 * 1024 * 1024;
pub const SCRCPY_SERVER_VERSION: &str = "3.3.3";
pub const SCRCPY_SERVER_CLASS_NAME: &str = "com.genymobile.scrcpy.Server";
pub const SCRCPY_SERVER_PATH: &str = "/data/local/tmp/scrcpy-server.jar";
pub const GRACEFUL_WAIT_MS: u64 = 250;
