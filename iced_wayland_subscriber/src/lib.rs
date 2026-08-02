mod info;
mod worker;

pub mod output;
pub mod shell;

#[cfg(feature = "workspace")]
pub mod workspace;

pub use info::{OutputId, OutputInfo, pixel_size};
pub use worker::Error;

use std::hash::Hash;
use std::os::fd::{AsFd, AsRawFd};
use wayland_client::Connection;

/// A [`Connection`] usable as a subscription key
#[derive(Debug, Clone)]
pub(crate) struct HashConnection {
    conn: Connection,
}

impl HashConnection {
    pub(crate) fn into_inner(self) -> Connection {
        self.conn
    }
}

impl Hash for HashConnection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.conn.as_fd().as_raw_fd().hash(state);
    }
}

impl From<Connection> for HashConnection {
    fn from(value: Connection) -> Self {
        Self { conn: value }
    }
}
