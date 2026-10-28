mod convert;
mod encrypt;

use std::path::PathBuf;

use crate::proxy::prelude::*;

// Repeater connection struct
pub struct RepeaterConn<T> {
    pub path: PathBuf,
    pub stream: T,
    pub update: bool,
    server_info: ServerInfo,
}

impl<T> RepeaterConn<T> {
    pub fn tls(&self) -> bool {
        self.server_info.is_tls()
    }

    pub fn address(&self) -> &Address {
        self.server_info.address()
    }
}
