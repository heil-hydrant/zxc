use tracing::trace;

use crate::interceptor::{InterUIOps, InterceptorHandler};
use crate::run::boundary::{HandleCommander, NotifyCommander};

impl NotifyCommander for InterceptorHandler {
    async fn notify_commander(
        &mut self,
    ) -> Result<(), <Self as HandleCommander>::Error> {
        if self.intercept_state {
            trace!("commander notified");
            self.intercept_state = false;
            let msg = InterUIOps::Toggle;
            self.to_commander.send(msg).await?;
        }
        Ok(())
    }
}
