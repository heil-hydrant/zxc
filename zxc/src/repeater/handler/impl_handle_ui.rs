use serde_json::json;
use tracing::error;

use crate::repeater::prelude::*;
use crate::run::boundary::HandleUI;

/* Steps:
 *      1. Parse the buf into RepeaterMsg.
 *
 *      2. If msg is to_repeater set id to 0 or set to msg.id to reuse in
 *         response.
 *
 *      3. To get response, call get_response() with msg as args.
 *          a. If response is Some, convert to string
 *          b. If response is Err, construct a json with error string.
 *
 *      4. If Some(data) is returned, return (id, data)
 *
 * Error:
 *      RepeaterError::MsgSerializing   [1]
 */

impl HandleUI for RepeaterHandler {
    async fn handle_ui(
        &mut self,
    ) -> Result<Option<(usize, String)>, RepeaterError> {
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
            return Ok(Some((id, data)));
        }
        Ok(None)
    }
}
