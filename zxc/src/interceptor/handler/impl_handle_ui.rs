use crate::commander::perform_codec_op;
use crate::interceptor::{
    InterUIOps, InterUImsg, InterceptorError, InterceptorHandler
};
use crate::run::boundary::HandleUI;

impl HandleUI for InterceptorHandler {
    async fn handle_ui(
        &mut self,
    ) -> Result<Option<(usize, String)>, InterceptorError> {
        let msg = serde_json::from_slice::<InterUImsg>(&self.buf)
            .map_err(|e| InterceptorError::from((&self.buf, e)))?;
        let id = msg.id;
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

            InterUIOps::Close => return Err(InterceptorError::UIclosed),
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
            return Ok(Some((id, result.to_string())));
        }
        Ok(None)
    }
}
