use super::AddonHandler;
use crate::forward_info::ForwardInfo;
use crate::run::boundary::FromCommander;

impl FromCommander for AddonHandler {
    type Message = ForwardInfo;

    async fn recv(&mut self) -> Option<Self::Message> {
        self.from_commander.recv().await
    }
}
