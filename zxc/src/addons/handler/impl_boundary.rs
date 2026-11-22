use super::AddonHandler;
use crate::addons::messages::AddonUiMsg;
use crate::addons::AddonError;
use crate::forward_info::ForwardInfo;
use crate::io::UnixSockError;
use crate::run::boundary::*;

impl FromCommander for AddonHandler {
    type Message = ForwardInfo;

    async fn recv(&mut self) -> Option<Self::Message> {
        self.from_commander.recv().await
    }
}

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

// Only Close msg is supported
impl HandleUI for AddonHandler {
    async fn handle_ui(&mut self) -> Result<UiReply, AddonError> {
        let _: AddonUiMsg = serde_json::from_slice(&self.buf)?;
        Err(UnixSockError::Closed.into())
    }
}
