mod convert;
mod encrypt;

use std::path::PathBuf;

use crate::forward_info::H2Info;
use crate::proxy::prelude::*;

// Repeater connection struct
pub struct RepeaterConn<T> {
    pub path: PathBuf,
    pub stream: T,
    pub update: bool,
    pub is_h2: bool,
    pub h2_info: Option<H2Info>,
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
