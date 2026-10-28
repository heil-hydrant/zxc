use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Receiver;

use crate::repeater::prelude::*;

/* Description:
 *      Trait Implementation types to RepeaterWs.
 *      Trait, not method for future-proofing (H2).
 *
 * Implementation:
 *      repeater/http/roneonestruct/impl_to_rws
 *
 * Returns:
 *      Ok(RWebSocket)
 *
 * Error:
 *      WsCreationError
 */

pub trait ToRepeaterWs<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    async fn to_rws(
        self,
        receiver: Receiver<RepeaterWsMsg>,
    ) -> Result<RWebSocket<T>, RepeaterError>;
}
