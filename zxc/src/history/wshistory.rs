use std::io::Error;

use tokio::fs::{File, OpenOptions};
use zxc_derive::Id;

use crate::history::HistoryWsRegisterInfo;
use crate::id::{Id, SoldierId};

pub const HISTORY_WS_WSESS: &str = "history.wsess";
pub const HISTORY_WS_HIS: &str = "./ws.whis";

// struct to map ws conn to .wsess file
#[derive(Id)]
pub struct WsHistory {
    id: SoldierId,
    file: File,
}

impl WsHistory {
    pub async fn new(mut info: HistoryWsRegisterInfo) -> Result<Self, Error> {
        info.path.push(HISTORY_WS_WSESS);
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&info.path)
            .await?;
        Ok(Self {
            id: info.id,
            file,
        })
    }

    pub fn file_as_mut(&mut self) -> &mut File {
        &mut self.file
    }
}
