pub mod builder;
pub mod convert;
pub mod repeater_ws_handle;
pub mod rwstruct;

use std::io::SeekFrom;

pub use builder::*;
pub use convert::*;
use futures_util::{SinkExt, StreamExt};
use header_plz::{OneInfoLine, OneResponseLine};
pub use repeater_ws_handle::*;
pub use rwstruct::*;
pub use super::messages::RepeaterWsMsg;
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt,
};
use tokio::sync::mpsc::{self, Receiver};
use tokio::{select, spawn};
use tokio_tungstenite::tungstenite::Message;
use tracing::{Instrument, Level, error, span, trace};

use crate::async_step::AsyncStep;
use crate::io::*;
use crate::proxy::*;
use crate::repeater::prelude::*;

/// Spawns a WebSocket task for an upgraded repeater connection.
///
/// # Steps
///
/// 1. Call `handle_http()` with the connection and get the response.
/// 2. Take the response and check the status code.
/// 3. If the status code is `101`, build an `mpsc` channel of
///    `RepeaterWsMessage`.
/// 4. Spawn `repeater_handle_ws()` with the receiver and connection.
/// 5. Build a `RepeaterWsHandle` with the index, sender, and task,
///    then return it.
/// 6. If the status code is not `101`, return
///    `RepeaterError::WsWrongScode`.
///
/// # Errors
///
/// `RepeaterError::WsWrongScode` [6].
pub async fn spawn_repeater_ws<T>(
    conn: RepeaterConn<T>,
    index: usize,
) -> Result<RepeaterWsHandle, RepeaterError>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin + Send + 'static,
{
    let mut hconn = handle_http(conn).await?;
    let response = hconn.get_payload().unwrap();
    let scode = OneResponseLine::try_build_infoline(response)?
        .status()?
        .into();
    match scode {
        101 => {
            let span = span!(Level::INFO, "repeater_ws", index);
            let (tx, rx) = mpsc::channel::<RepeaterWsMsg>(1);
            let task = spawn(async move {
                if let Err(e) = repeater_handle_ws(hconn, rx)
                    .instrument(span)
                    .await
                {
                    error!("ws handle| {:?}", e);
                }
            });
            let handle = RepeaterWsHandle::new(index, tx, task);
            Ok(handle)
        }
        scode => Err(RepeaterError::WsWrongScode(scode)),
    }
}

/// Handles the WebSocket repeater task.
///
/// # Steps
///
/// 1. Convert `T` into `RWebSocket`.
/// 2. Loop on `select`:
///   - Server side: read from the server, write the response, and log.
///   - Client side: receive a message from the repeater UI, write it to
///     the server, and log.
pub async fn repeater_handle_ws<T, E>(
    conn: T,
    receiver: Receiver<RepeaterWsMsg>,
) -> Result<(), RepeaterError>
where
    T: ToRepeaterWs<E>,
    E: AsyncRead + AsyncWrite + Unpin,
{
    // 1. convert <T> to RWebSocket
    let mut rws = conn.to_rws(receiver).await?;
    trace!("ws start");
    // 2. Loop
    loop {
        select! {
            // 2.a.1 Read from server
            result = rws.stream.next() => {
                match result {
                    Some(frame) => {
                        rws.add_frame(frame.map_err(WsError::Read)?);
                        trace!("from server");
                        // 2.a.3 Log server
                        rws = log_response(rws).await?;
                        rws.log(Role::Client).await?;
                    }
                    None => {
                        trace!("server closed ws stream");
                        return Ok(());
                    }
                }
            }
            // 2.b.1 Msg from repeater ui
            result = rws.receiver.recv() => {
                match result {
                    Some(msg) => {
                        trace!("from client");
                        match msg {
                            RepeaterWsMsg::Read { is_bin } => {
                                rws.set_is_bin(is_bin);
                                let mut state = RepeaterState::ReadFromFile(rws);
                                rws = loop {
                                    state = state.next().await?;
                                    match state {
                                        // Read from server break
                                        RepeaterState::Receive(conn) => break conn,
                                        _ => continue
                                    }
                                };
                                // 2.b.3 Log client
                                rws.log(Role::Server).await?;
                                rws.file_as_mut().seek(SeekFrom::Start(0)).await.
                                    map_err(|e| FileErrorInfo::from((rws.file_as_mut(),
                                    FileEvent::Seek, e)))?;
                            }
                            RepeaterWsMsg::Close => {
                                let close_msg = Message::Close(None);
                                rws.stream.send(close_msg).await.map_err(|_| WsError::Write)?;
                                trace!("cls frame sent");
                                return Ok(());
                            }
                        }
                    }
                    None => {
                        trace!("repeater ui receiver dropped");
                        return Ok(());
                    }
                }
            }
        }
    }
}
