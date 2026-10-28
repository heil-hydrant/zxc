use super::AddonHandler;
use crate::addons::AddonError;
use crate::addons::message::from_ui::AddonMsg;
use crate::io::UnixSockError;
use crate::run::boundary::HandleUI;

// Only Close msg is supported
impl HandleUI for AddonHandler {
    async fn handle_ui(
        &mut self,
    ) -> Result<Option<(usize, String)>, AddonError> {
        let _: AddonMsg = serde_json::from_slice(&self.buf)?;
        Err(UnixSockError::Closed.into())
    }
}
