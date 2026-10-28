use std::ffi::OsStr;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::proxy::server_info::json::ServerInfoJson;

#[derive(Serialize, Deserialize, Debug)]
pub enum Module {
    Repeater,
    Addon(String),
}

// Message that is sent between UI
#[derive(Serialize, Deserialize, Debug)]
pub struct ForwardInfo {
    #[serde(skip_serializing)]
    to: Module,
    pub file: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfoJson>,
}

impl ForwardInfo {
    pub fn to_module(&self) -> &Module {
        &self.to
    }

    pub fn file_extension(&self) -> Option<&OsStr> {
        self.file.extension()
    }
}
