use std::path::PathBuf;

use serde::Deserialize;

use crate::forward_info::ForwardInfo;
use crate::id::SoldierId;
use crate::proxy::ServerInfo;

// ============================================================================
// Internal IPC Messages (Commander to History)
// ============================================================================

// Messages from commander to history
pub enum CommanderToHistory {
    // From Commander - http
    Http(String),
    // From Soldiers - ws
    RegisterWs(HistoryWsRegisterInfo),
    WebSocket(SoldierId, String),
    RemoveWs(SoldierId),
}

// Ws Register
pub struct HistoryWsRegisterInfo {
    pub id: SoldierId,
    http_id: usize,
    pub path: PathBuf,
    server_info: ServerInfo,
}

impl HistoryWsRegisterInfo {
    pub fn new(
        id: SoldierId,
        http_id: usize,
        path: PathBuf,
        server_info: ServerInfo,
    ) -> Self {
        Self {
            id,
            http_id,
            path,
            server_info,
        }
    }

    pub fn log_data(&self) -> String {
        format!(
            "{0} | {1} | {2}\n",
            self.http_id,
            self.server_info.scheme(),
            self.server_info.address()
        )
    }
}

// ============================================================================
// Frontend IPC Messages (From History UI)
// ============================================================================

// Messages from history UI
#[derive(Debug, Deserialize)]
pub struct HistoryUImsg {
    _id: usize,
    pub operation: HistoryUIOps,
}

// History UI Operations
#[derive(Deserialize, Debug)]
pub enum HistoryUIOps {
    Close,
    Forward(ForwardInfo),
    ReloadConfig,
}

impl From<HistoryUImsg> for HistoryUIOps {
    fn from(msg: HistoryUImsg) -> Self {
        msg.operation
    }
}
