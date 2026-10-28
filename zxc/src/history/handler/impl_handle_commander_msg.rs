use crate::history::{HistoryError, HistoryHandler};
use crate::run::boundary::HandleCommander;

impl HandleCommander for HistoryHandler {
    type Error = HistoryError;

    async fn handle_commander_no_ui(
        &mut self,
        size: usize,
    ) -> Result<(), Self::Error> {
        self.handle_commander(size, false)
            .await?;
        Ok(())
    }

    async fn handle_commander_ui(
        &mut self,
        size: usize,
    ) -> Result<Option<String>, Self::Error> {
        if let Some(result) = self
            .handle_commander(size, true)
            .await?
        {
            return Ok(Some(result));
        };
        Ok(None)
    }
}
