use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Receiver;

use crate::repeater::prelude::*;

/// Converts connection types into `RepeaterWs` implementations.
///
/// A trait instead of a method to leave room for HTTP/2.
///
/// # Errors
///
/// `WsCreationError`.
pub trait ToRepeaterWs<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    async fn to_rws(
        self,
        receiver: Receiver<RepeaterWsMsg>,
    ) -> Result<RWebSocket<T>, RepeaterError>;
}
