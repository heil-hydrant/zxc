use crate::interceptor::{
    InterToUI, InterUIOps, InterceptorError, InterceptorHandler
};
use crate::run::boundary::HandleCommander;

impl HandleCommander for InterceptorHandler {
    type Error = InterceptorError;

    async fn handle_commander_no_ui(
        &mut self,
        _msg: InterToUI,
    ) -> Result<(), Self::Error> {
        if self.intercept_state {
            self.intercept_state = false;
            self.to_commander
                .send(InterUIOps::Toggle)
                .await?
        }
        Err(InterceptorError::NoUI)
    }

    async fn handle_commander_ui(
        &mut self,
        info: InterToUI,
    ) -> Result<Option<String>, Self::Error> {
        Ok(Some(serde_json::to_string(&info)?))
    }
}
