use crate::history::HistoryWsRegisterInfo;
use crate::id::SoldierId;

// Messages from commander to history
pub enum CommanderToHistory {
    // From Commander - http
    Http(String),
    // From Soldiers - ws
    RegisterWs(HistoryWsRegisterInfo),
    WebSocket(SoldierId, String),
    RemoveWs(SoldierId),
}
