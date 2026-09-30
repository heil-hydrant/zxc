use tokio::sync::mpsc::Sender;

use super::*;
use crate::history::message::from_commander::CommanderToHistory;
use crate::proxy::handler_state::transition::write_history::SendHistory;

impl<T, E, U> SendHistory for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    fn get_sender(&self) -> &Sender<CommanderToHistory> {
        self.history_sendr.as_ref().unwrap()
    }
}
