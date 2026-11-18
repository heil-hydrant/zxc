use crate::history::{
    HistoryError, HistoryHandler, HistoryUIOps, HistoryUImsg
};
use crate::io::UnixSockError;
use crate::run::boundary::{HandleUI, UiReply};

// Handles one decoded UI message for history.
//
// # Steps
//
// 1. Deserialize the buffer to `HistoryUImsg`.
// 2. If the operation is close, return `HistoryError::UI`.
// 3. Otherwise send the operation to the commander.
//
// # Errors
//
// - `HistoryError::MsgDecode` [1]
// - `HistoryError::UI` [2]
// - `HistoryError::CommanderSend` [3]
impl HandleUI for HistoryHandler {
    async fn handle_ui(&mut self) -> Result<UiReply, HistoryError> {
        let msg = serde_json::from_slice::<HistoryUImsg>(&self.buf)?;
        if matches!(msg.operation, HistoryUIOps::Close) {
            return Err(UnixSockError::Closed.into());
        }
        self.to_commander
            .send(msg.into())
            .await?;
        Ok(UiReply::None)
    }
}
