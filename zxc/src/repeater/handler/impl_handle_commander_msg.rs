use crate::file_types::{EXT_REQ, EXT_WREQ};
use crate::forward_info::{ForwardInfo, Module};
use crate::repeater::prelude::*;
use crate::run::boundary::{FromCommander, HandleCommander};

impl HandleCommander for RepeaterHandler {
    type Error = RepeaterError;

    async fn handle_commander_no_ui(
        &mut self,
        _: <Self as FromCommander>::Message,
    ) -> Result<(), Self::Error> {
        Err(RepeaterError::NoUI)
    }

    /* Steps:
     *      1. If msg is to repeater,
     *          a. get extension
     *          b. build dest file based on extension
     *                  req  => build_repeater_dest_req
     *                  wreq => build_repeater_dest_ws
     *
     *      2. set the msg file to dest file
     *      3. serialize to string and return
     *      4. if msg is not for repeater, send to commander
     *
     *  Errors:
     *      RepeaterError
     *          NoExtension         [1.a]
     *          UnknownExtension    [1.b]
     *          MsgSerializing      [3]
     *          CommanderSend       [4]
     */
    async fn handle_commander_ui(
        &mut self,
        mut msg: ForwardInfo,
    ) -> Result<Option<String>, Self::Error> {
        if let Module::Repeater = msg.to_module() {
            let ext = msg
                .file_extension()
                .ok_or(RepeaterError::NoExtension(msg.file.clone()))?
                .to_str()
                .ok_or(RepeaterError::NoExtension(msg.file.clone()))?;
            msg.file = match ext {
                EXT_REQ => build_repeater_dest(&msg.file, ext).await?,
                EXT_WREQ => build_repeater_dest_ws(&mut msg).await?,
                _ => Err(RepeaterError::UnknownExtension(msg.file.clone()))?,
            };
            return Ok(Some(serde_json::to_string(&msg)?));
        }
        self.to_commander.send(msg).await?;
        Ok(None)
    }
}
