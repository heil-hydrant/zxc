use std::path::PathBuf;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::history::{CommanderToHistory, HistoryUIOps};

pub struct HistoryComm {
    pub from_history: Receiver<HistoryUIOps>,
    pub to_history: Sender<CommanderToHistory>,
    history_path: PathBuf,
    pub http_log_index: usize,
}

impl HistoryComm {
    #[inline(always)]
    pub fn new(
        http_log_index: usize,
        from_history: Receiver<HistoryUIOps>,
        to_history: Sender<CommanderToHistory>,
    ) -> Self {
        Self {
            from_history,
            to_history,
            history_path: PathBuf::from("./history"),
            http_log_index,
        }
    }

    pub fn path(&self) -> PathBuf {
        self.history_path.clone()
    }
}
