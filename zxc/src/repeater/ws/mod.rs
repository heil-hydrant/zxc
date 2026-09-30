use std::io::SeekFrom;

use futures_util::{SinkExt, StreamExt};
use header_plz::{OneInfoLine, OneResponseLine};
use repeater_ws_handle::RepeaterWsHandle;
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt,
};
use tokio::sync::mpsc::{self, Receiver};
use tokio::{select, spawn};
use tokio_tungstenite::tungstenite::Message;
use tracing::{Instrument, Level, error, span, trace};

use crate::async_step::AsyncStep;
use crate::io::file::{FileErrorInfo, FileEvent};
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::handler_state::handlers::error::WsError;
use crate::proxy::handler_state::role::Role;
use crate::repeater::states::rstate::RepeaterState;
use crate::repeater::states::transition::write_response::log_response;
pub mod builder;
pub mod convert;
pub mod message;
pub mod repeater_ws_handle;
pub mod rwstruct;
use convert::*;
use message::*;

use super::conn::RepeaterConn;
use super::error::RepeaterError;
use super::http::handle_http;

/* Description:
 *      Function to spawn a ws task
 *
 * Steps:
 *      1. call handle_http() with conn and get response.
 *      2. take the response and check status code.
 *      3. If status code is 101, build mpsc<RepeaterWsMessage>
 *      4. Spawn task to handle ws, repeater_handle_ws() with
 *        mpsc_receiver<RepeaterWsMessage> and conn
 *      5. Build new RepeaterWsHandle with index, tx , task and return
 *      6. If status code is not 101, return RepeaterError::WsWrongScode
 *
 * Error:
 *      RepeaterError::WsWrongScode     [6]
 */

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
            let _ = span.enter();
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

/* Description:
 *      Handler Function for Repeater ws
 *
 * Steps:
 *     1. Convert type T to RWebSocket
 *     2. ----- loop -----
 *          ----- select -----
 *          a. Server Side
 *             1. Read from server
 *             2. Write response
 *             3. Log
 *          b. Client Side
 *             1. Msg from repeater ui
 *             2. Write to server
 *             3. Log
 */
pub async fn repeater_handle_ws<T, E>(
    conn: T,
    receiver: Receiver<RepeaterWsMsg>,
) -> Result<(), ProxyStateError>
where
    T: ToRepeaterWs<E>,
    E: AsyncRead + AsyncWrite + Unpin,
{
    // 1. convert <T> to RWebSocket
    let mut rws = conn.to_rws(receiver).await.unwrap();
    trace!("ws start");
    // 2. Loop
    loop {
        select! {
            // 2.a.1 Read from server
            result = rws.stream.next() => {
                if let Some(frame) = result {
                    rws.add_frame(frame.map_err(WsError::Read)?);
                    trace!("from server");
                    // 2.a.3 Log server
                    rws = log_response(rws)
                        .await
                        .map_err(ProxyStateError::ReWriteFile)?;
                    rws.log(Role::Client).await?;
                }
            }
            // 2.b.1 Msg from repeater ui
            result = rws.receiver.recv() => {
                if let Some(msg) = result {
                    trace!("from client");
                    match msg {
                        RepeaterWsMsg::Read => {
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
            }
        }
    }
}
