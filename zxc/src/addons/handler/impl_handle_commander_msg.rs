use super::AddonHandler;
use crate::addons::AddonError;
use crate::forward_info::ForwardInfo;
use crate::run::boundary::{HandleCommander, UiError, UiReply};

impl HandleCommander for AddonHandler {
    type Error = AddonError;

    #[inline(always)]
    async fn handle_commander_no_ui(
        &mut self,
        _: ForwardInfo,
    ) -> Result<(), Self::Error> {
        Err(AddonError::from(UiError::NoUI))
    }

    async fn handle_commander_ui(
        &mut self,
        msg: ForwardInfo,
    ) -> Result<UiReply, Self::Error> {
        let cmd = self.build_cmd_string(msg).await?;
        Ok(UiReply::push(cmd))
    }
}
