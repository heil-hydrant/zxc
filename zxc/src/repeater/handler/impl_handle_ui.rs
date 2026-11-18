use serde_json::json;
use tracing::error;

use crate::repeater::prelude::*;
use crate::run::boundary::{HandleUI, UiReply};

// Handles one decoded UI message for the repeater.
//
// # Steps
//
// 1. Parse the buffer into `RepeaterMsg`.
// 2. Set the reply ID to `0` for repeater messages, otherwise reuse
//    `msg.id`.
// 3. Call `get_response()` with the message.
//    a. A `Some` response is converted to a string.
//    b. An `Err` response becomes a JSON object with the error string.
// 4. Return `(id, data)` when a response is produced.
//
// # Errors
//
// `RepeaterError::MsgSerializing` [1].

impl HandleUI for RepeaterHandler {
    async fn handle_ui(&mut self) -> Result<UiReply, RepeaterError> {
        let msg = serde_json::from_slice::<RepeaterMsg>(&self.buf)?; // 1
        let id = if msg.to_repeater() {
            0
        } else {
            msg.id
        };
        let to_send = match self.get_response(msg).await {
            Ok(Some(res)) => Some(res),
            Err(e) => {
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
