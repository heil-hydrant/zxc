mod history_struct;

pub use history_struct::*;
use tokio::sync::mpsc::Sender;
use tracing::trace;

use crate::history::CommanderToHistory;
use crate::proxy::prelude::*;

pub trait GetHistory {
    fn get_history(&self) -> HistoryEnum<'_>;
}

pub trait SendHistory {
    fn get_sender(&self) -> &Sender<CommanderToHistory>;
}

pub async fn write_history<T>(
    conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: GetHistory + SendHistory,
{
    let history: CommanderToHistory = conn.get_history().try_into()?;
    conn.get_sender().send(history).await?;
    trace!("history written");
    Ok(ProxyState::Log(conn))
}
