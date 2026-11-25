use tracing::trace;

use super::InterceptorHandler;
use crate::commander::perform_codec_op;
use crate::interceptor::{
    InterToUI, InterUIOps, InterUImsg, InterceptorError,
};
use crate::run::boundary::{
    FromCommander, HandleCommander, HandleUI, NotifyCommander, UiError,
    UiReply,
};

impl FromCommander for InterceptorHandler {
    type Message = InterToUI;

    async fn recv(&mut self) -> Option<Self::Message> {
        self.from_commander.recv().await
    }
}

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
                .await?;
        }
        Err(InterceptorError::from(UiError::NoUI))
    }

    async fn handle_commander_ui(
        &mut self,
        info: InterToUI,
    ) -> Result<UiReply, Self::Error> {
        Ok(UiReply::push(serde_json::to_string(&info)?))
    }
}

impl HandleUI for InterceptorHandler {
    async fn handle_ui(&mut self) -> Result<UiReply, InterceptorError> {
        let msg = serde_json::from_slice::<InterUImsg>(&self.buf)
            .map_err(|e| InterceptorError::from((&self.buf, e)))?;
        let id = msg.id();
        let result = match msg.op() {
            InterUIOps::Toggle
            | InterUIOps::Resume(_)
            | InterUIOps::Forward(_)
            | InterUIOps::Drop(..) => {
                if matches!(msg.op(), InterUIOps::Toggle) {
                    self.intercept_state = !self.intercept_state;
                }
                self.to_commander
                    .send(msg.into())
                    .await?;
                None
            }

            InterUIOps::Close => {
                return Err(InterceptorError::from(UiError::Closed));
            }
            InterUIOps::Encode {
                codec,
                data,
            } => {
                let result = perform_codec_op(true, codec, data.as_bytes())?;
                Some(result)
            }
            InterUIOps::Decode {
                codec,
                data,
            } => {
                let result = perform_codec_op(false, codec, data.as_bytes())?;
                Some(result)
            }
        };
        if let Some(result) = result {
            return Ok(UiReply::Reply(id, result.to_string()));
        }
        Ok(UiReply::None)
    }
}

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
