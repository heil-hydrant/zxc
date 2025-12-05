mod history_struct;
pub use history_struct::*;
use tokio::sync::mpsc::Sender;
use tracing::trace;

use crate::history::message::from_commander::CommanderToHistory;
use crate::proxy::handler_state::{ProxyState, ProxyStateError};

pub trait GetHistory {
    fn get_history(&self) -> HistoryEnum<'_>;
}

pub trait SendHistory {
    fn get_sender(&self) -> &Sender<CommanderToHistory>;
}

/* Description:
 *      Transition function to send the http history to commander or ws history
 *      to history task.
 *
 * Transition:
 *      WriteHistory -> Log
 *
 * Steps:
 *      1. Get the history data and convert to CommanderToHistory.
 *      2. Get Sender<CommanderToHistory> and send the data.
 *
 * Returns:
 *      Ok(ProxyState::Log)
 *
 * Error:
 *      ProxyStateError::Serialize      [1]
 *      ProxyStateError::HistorySend    [2]
 */

pub async fn write_history<T>(
    conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: GetHistory + SendHistory,
{
    let history: CommanderToHistory = conn.get_history().try_into()?;
    conn.get_sender().send(history).await?;
    trace!("Y");
    Ok(ProxyState::Log(conn))
}
