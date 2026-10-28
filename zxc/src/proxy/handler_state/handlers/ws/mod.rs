pub mod error;
mod wsconn;

use std::marker::Send;

use convert::ToWs;
pub use error::*;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{Instrument, Level, span, trace};
pub use wsconn::*;

use crate::async_step::async_run;
use crate::proxy::prelude::*;

pub async fn handle_websocket<T, E, U>(val: T) -> Result<(), ProxyStateError>
where
    T: ToWs<E, U>,
    E: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
    U: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
{
    trace!("[+] ws start");
    let (server, client): (WsConn<U, E>, WsConn<E, U>) =
        val.convert()
            .await
            .map_err(WsError::Create)?;

    let clientstate = ProxyState::Receive(client);
    let serverstate = ProxyState::Receive(server);

    let client_span = span!(Level::TRACE, "client");
    let _ = client_span.enter();
    let client_flow = async_run(clientstate).instrument(client_span);

    let server_span = span!(Level::TRACE, "server");
    let _ = server_span.enter();
    let server_flow = async_run(serverstate).instrument(server_span);

    tokio::select! {
        result = client_flow => {
            result?;
        }
        result = server_flow => {
            result?;
        }
    }

    trace!("[+] ws end");
    Ok(())
}
