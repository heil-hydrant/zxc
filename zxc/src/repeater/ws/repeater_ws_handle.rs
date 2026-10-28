use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinHandle;

use crate::repeater::prelude::*;

/* Description:
 *       Handle for RepeaterWs.
 *       Sender is used for sending messages from repeater to ws task.
 *
 * Flow:
 *     ┌───────────┐     ┌───────────────────┐    ┌───────┐
 *     │repeater_ui├────►│repeater_main_task ├───►│ws-task│
 *     └───────────┘     └───────────────────┘    └───────┘
 */

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
