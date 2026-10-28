use crate::history::{
    HistoryError, HistoryHandler, HistoryUIOps, HistoryUImsg
};
use crate::io::UnixSockError;
use crate::run::boundary::HandleUI;

/* Steps:
 *      1. Deserialize the buffer to HistoryUImsg
 *      2. If operation is close, return HistoryError::UI
 *      3. Else send operation to commander
 *
 * Error:
 *      HistoryError::MsgDecode         [1]
 *      HistoryError::UI                [2]
 *      HistoryError::CommanderSend     [3]
 */

impl HandleUI for HistoryHandler {
    async fn handle_ui(
        &mut self,
    ) -> Result<Option<(usize, String)>, HistoryError> {
        let msg = serde_json::from_slice::<HistoryUImsg>(&self.buf)?;
        if matches!(msg.operation, HistoryUIOps::Close) {
            return Err(UnixSockError::Closed.into());
        }
        self.to_commander
            .send(msg.into())
            .await?;
        Ok(None)
    }
}
