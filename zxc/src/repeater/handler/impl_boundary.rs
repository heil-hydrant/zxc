use serde_json::json;
use tracing::error;

use crate::file_types::{EXT_REQ, EXT_WREQ};
use crate::forward_info::{ForwardInfo, Module};
use crate::repeater::messages::RepeaterMsg;
use crate::repeater::prelude::*;
use crate::run::boundary::{
    FromCommander, HandleCommander, HandleUI, IsUIError, UiError, UiReply,
};

impl FromCommander for RepeaterHandler {
    type Message = ForwardInfo;

    async fn recv(&mut self) -> Option<Self::Message> {
        self.from_commander.recv().await
    }
}

impl HandleCommander for RepeaterHandler {
    type Error = RepeaterError;

    async fn handle_commander_no_ui(
        &mut self,
        _: <Self as FromCommander>::Message,
    ) -> Result<(), Self::Error> {
        Err(RepeaterError::from(UiError::NoUI))
    }

    // Forwards a `ForwardInfo` to the repeater or back to the commander.
    async fn handle_commander_ui(
        &mut self,
        mut msg: ForwardInfo,
    ) -> Result<UiReply, Self::Error> {
        if let Module::Repeater = msg.to_module() {
            let ext = msg
                .file_extension()
                .ok_or_else(|| RepeaterError::NoExtension(msg.file.clone()))?
                .to_str()
                .ok_or_else(|| RepeaterError::NoExtension(msg.file.clone()))?;
            msg.file = match ext {
                EXT_REQ => build_repeater_dest(&msg.file, ext).await?,
                EXT_WREQ => build_repeater_dest_ws(&mut msg).await?,
                _ => Err(RepeaterError::UnknownExtension(msg.file.clone()))?,
            };
            return Ok(UiReply::push(serde_json::to_string(&msg)?));
        }
        self.to_commander.send(msg).await?;
        Ok(UiReply::None)
    }
}

// Handles one decoded UI message for the repeater.
impl HandleUI for RepeaterHandler {
    async fn handle_ui(&mut self) -> Result<UiReply, RepeaterError> {
        let msg = serde_json::from_slice::<RepeaterMsg>(&self.buf)?;
        let id = if msg.to_repeater() {
            0
        } else {
            msg.id
        };
        let to_send = match self.get_response(msg).await {
            Ok(Some(res)) => Some(res),
            Err(e) => {
                if e.is_ui_error() {
                    return Err(e);
                }
                error!("{}", e);
                Some(json!({"error" : e.to_string()}).to_string())
            }
            Ok(None) => None,
        };
        if let Some(data) = to_send {
            return Ok(UiReply::Reply(id, data));
        }
        Ok(UiReply::None)
    }
}
