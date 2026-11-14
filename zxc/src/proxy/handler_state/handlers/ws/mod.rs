pub mod error;
mod wsconn;

use std::marker::Send;
use std::time::Duration;

use convert::ToWs;
pub use error::*;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use tracing::{Instrument, Level, span, trace};
pub use wsconn::*;

use crate::async_step::async_run;
use crate::proxy::prelude::*;

const WS_CLOSE_TIMEOUT: Duration = Duration::from_secs(1);

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
    let client_flow = async move {
        async_run(clientstate)
            .instrument(client_span)
            .await?;
        Ok::<(), ProxyStateError>(())
    };

    let server_span = span!(Level::TRACE, "server");
    let _ = server_span.enter();
    let server_flow = async move {
        async_run(serverstate)
            .instrument(server_span)
            .await?;
        Ok::<(), ProxyStateError>(())
    };

    tokio::pin!(client_flow, server_flow);

    let (completed, role) = tokio::select! {
        result = &mut client_flow => (result, Role::Client),
        result = &mut server_flow => (result, Role::Server),
    };
    completed?;

    let to_complete = match role {
        Role::Client => timeout(WS_CLOSE_TIMEOUT, &mut server_flow).await,
        Role::Server => timeout(WS_CLOSE_TIMEOUT, &mut client_flow).await,
    };

    match to_complete {
        Ok(result) => result?,
        Err(_) => trace!("ws close timeout"),
    }

    trace!("[+] ws end");
    Ok(())
}
