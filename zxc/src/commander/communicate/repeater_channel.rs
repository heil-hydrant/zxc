use tokio::sync::mpsc::{Receiver, Sender};

use crate::forward_info::ForwardInfo;

pub struct RepeaterComm {
    pub from_repeater: Receiver<ForwardInfo>,
    pub to_repeater: Sender<ForwardInfo>,
}

impl RepeaterComm {
    #[inline(always)]
    pub fn new(
        from_repeater: Receiver<ForwardInfo>,
        to_repeater: Sender<ForwardInfo>,
    ) -> Self {
        Self {
            from_repeater,
            to_repeater,
        }
    }
}
