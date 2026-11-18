use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinHandle;

use crate::repeater::prelude::*;

/// Handle for a `RepeaterWs` task.
///
/// The sender carries messages from the repeater to the WebSocket task.
///
/// The flow is `repeater_ui` -> `repeater_main_task` -> `ws-task`.
pub struct RepeaterWsHandle {
    pub id: usize,
    sender: Sender<RepeaterWsMsg>,
    pub handle: JoinHandle<()>,
}

impl RepeaterWsHandle {
    pub fn new(
        id: usize,
        sender: Sender<RepeaterWsMsg>,
        handle: JoinHandle<()>,
    ) -> Self {
        RepeaterWsHandle {
            id,
            sender,
            handle,
        }
    }

    // Send a message to ws task
    pub async fn send(
        &self,
        msg: RepeaterWsMsg,
    ) -> Result<(), SendError<RepeaterWsMsg>> {
        self.sender.send(msg).await
    }
}
