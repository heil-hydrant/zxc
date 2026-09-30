pub mod error;
use convert::ToWs;
use error::*;
mod wstruct;
use std::marker::Send;

use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{Instrument, Level, span, trace};
pub use wstruct::*;

use crate::async_step::async_run;
use crate::proxy::handler_state::ProxyState;
use crate::proxy::handler_state::error::ProxyStateError;

/* Description:
 *      Function to handle websocket connection cycle.
 *
 * Args:
 *      T
 *
 * Trait Bound:
 *      T  : ToWs<E, U>
 *      E  : AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
 *      U  : AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
 *
 * Steps:
 *      1. Split T into client WsStruct and server WsStruct
 *      2. Create client and server states, ProxyState::Receive
 *      3. Run (async_run) client and server states concurrently in a
 *      select! block
 *
 * Returns:
 *      Ok(())
 *
 * Error:
 *      ProxyStateError
 */

pub async fn handle_websocket<T, E, U>(val: T) -> Result<(), ProxyStateError>
where
    T: ToWs<E, U>,
    E: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
    U: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
{
    trace!("ws start");
    let (server, client): (WsStruct<U, E>, WsStruct<E, U>) = val
        .convert()
        .await
        .map_err(WsError::Create)?;

    // 2. Create client and server states
    let clientstate = ProxyState::Receive(client);
    let serverstate = ProxyState::Receive(server);

    // 3. Run
    let cspan = span!(Level::TRACE, "client");
    let _ = cspan.enter();
    let client_flow = async_run(clientstate).instrument(cspan);

    let sspan = span!(Level::TRACE, "server");
    let _ = sspan.enter();
    let server_flow = async_run(serverstate).instrument(sspan);

    tokio::select! {
        result = client_flow => {
            result?;
        }
        result = server_flow => {
            result?;
        }
    }

    trace!("ws end");
    Ok(())
}
