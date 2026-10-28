use crate::forward_info::ForwardInfo;
use crate::repeater::prelude::*;
use crate::run::boundary::FromCommander;

impl FromCommander for RepeaterHandler {
    type Message = ForwardInfo;

    async fn recv(&mut self) -> Option<Self::Message> {
        self.from_commander.recv().await
    }
}
