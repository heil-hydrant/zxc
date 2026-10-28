mod twoconn;

use std::fmt::Debug;

use header_plz::{RequestLine, ResponseLine};
use http_plz::Request;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinSet;
use tracing::{Instrument, error, trace};
use two_plz::client::{ClientBuilder, SendRequest};
use two_plz::codec::UserError;
use two_plz::error::OpError;
use two_plz::preface::PrefaceError;
use two_plz::server::{SendResponse, ServerBuilder};
use twoconn::*;

use crate::async_step::async_run;
use crate::commander::*;
use crate::proxy::prelude::*;

#[derive(Debug, Error)]
pub enum Http2Error {
    #[error("preface| {0}")]
    Preface(#[from] PrefaceError),
    #[error("op| {0}")]
    Op(#[from] OpError),
    #[error("user| {0}")]
    User(#[from] UserError),
    #[error("proxy state| {0}")]
    Proxy(#[from] ProxyStateError),
    #[error("commander send| {0}")]
    CommanderSend(
        #[from] tokio::sync::mpsc::error::SendError<CommanderRequest>,
    ),
    #[error("unexpected h2 request state")]
    UnexpectedState,
    #[error("commander recv| {0}")]
    CommanderRecv(#[from] tokio::sync::oneshot::error::RecvError),
}

pub async fn handle_http2<T, E>(
    conn: Connection<T, E>,
    _recvr: Receiver<CommanderResponse>,
    server_info: ServerInfo,
) -> Result<(), Http2Error>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    let mut h2_tasks = JoinSet::new();
    let mut server_conn = ServerBuilder::new()
        .handshake(conn.reader)
        .await?;

    let (client_conn, client) = ClientBuilder::new()
        .handshake(conn.writer)
        .await?;

    h2_tasks.spawn(async move {
        if let Err(e) = client_conn.await {
            error!("h2 upstream connection driver error| {e}");
        }
    });

    let conn_id = conn.id;
    let commander_tx = conn.commander.clone();

    while let Some(result) = server_conn.accept().await {
        let (request, send_response) = match result {
            Ok(pair) => pair,
            Err(e) => {
                error!("h2 accept error| {e}");
                continue;
            }
        };

        let stream_id: u32 = send_response.stream_id().into();
        let soldier_id = SoldierId::H2Stream {
            conn_id,
            stream_id,
        };
        let send_request = client.clone();
        let commander_tx = commander_tx.clone();
        let server_info = server_info.clone();
        let stream_span = tracing::span!(
            tracing::Level::TRACE,
            "H2Stream",
            conn = conn_id,
            stream = stream_id
        );

        h2_tasks.spawn(
            async move {
                let res = handle_stream(
                    soldier_id,
                    request,
                    send_response,
                    send_request,
                    commander_tx.clone(),
                    server_info,
                )
                .await;

                let _ = commander_tx
                    .send(CommanderRequest::Close(soldier_id))
                    .await;

                if let Err(e) = res {
                    error!("h2 stream error| {e}");
                }
            }
            .instrument(stream_span),
        );
    }

    Ok(())
}

async fn handle_stream(
    soldier_id: SoldierId,
    request: Request,
    send_response: SendResponse,
    send_request: SendRequest,
    commander_tx: Sender<CommanderRequest>,
    server_info: ServerInfo,
) -> Result<(), Http2Error> {
    trace!("h2 stream start | {soldier_id}");
    let (tx, rx) = oneshot::channel();
    commander_tx
        .send(CommanderRequest::H2StreamRegister(soldier_id, tx))
        .await?;
    let commander_recvr = rx.await?;

    let h2req = TwoConn::<RequestLine>::new(
        soldier_id,
        request,
        send_response,
        send_request,
        commander_tx.clone(),
        commander_recvr,
        server_info,
    );

    let client_state = ProxyState::ShouldLog(h2req);
    let client_state = async_run(client_state).await?;
    let h2req = match client_state {
        ProxyState::End(conn) => conn,
        ProxyState::NewConnection(conn, _) => {
            // HTTP/2 reissue is not implemented yet. Ignore the requested
            // destination change and send on the existing upstream connection.
            trace!("h2 new connection ignored | {soldier_id}");
            match async_run(ProxyState::Send(conn)).await? {
                ProxyState::End(conn) => conn,
                _ => return Err(Http2Error::UnexpectedState),
            }
        }
        _ => return Err(Http2Error::UnexpectedState),
    };

    if let Some(path) = h2req.path.as_ref() {
        let info = ServerInfoJson::from(&h2req.server_info);
        if let Err(e) = set_attr(path, info) {
            error!("h2 set attr| {e}");
        }
    }

    let h2resp = TwoConn::<ResponseLine>::from(h2req);
    let server_state = ProxyState::Receive(h2resp);
    let server_state = async_run(server_state).await?;
    let _ = TwoConn::<ResponseLine>::try_from(server_state)?;

    Ok(())
}
