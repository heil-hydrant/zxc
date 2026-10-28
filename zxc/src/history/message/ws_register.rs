use std::path::PathBuf;

use crate::id::SoldierId;
use crate::proxy::ServerInfo;

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
