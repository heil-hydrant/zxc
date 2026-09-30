use std::path::PathBuf;

use bytes::Buf;

pub trait Log {
    fn path(&self) -> &PathBuf;

    fn log_data(&self) -> &[u8];
}

pub trait NLog {
    fn log_data(&self) -> impl Buf;
}
