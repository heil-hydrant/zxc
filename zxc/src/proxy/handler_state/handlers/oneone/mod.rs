use header_plz::body_headers::parse::ParseBodyHeaders;
use header_plz::{OneInfoLine, OneRequestLine, OneResponseLine};
use http_plz::{OneMessageHead, OneOne};
use oneone_plz::state::State;

use crate::CAPACITY_2MB;
use crate::async_step::async_run;
use crate::commander::{CommanderRequest, Protocol};
use crate::interceptor::message::from_ui::resume_info::ResumeInfo;
use crate::io::socket::fill_buffer;
use crate::proxy::handler_state::ProxyState;
use crate::proxy::handler_state::read_write::ReadWrite;
use crate::proxy::handler_state::transition::Log;
use crate::proxy::handler_state::transition::reconnect::Reconnect;
use crate::proxy::server_info::ServerInfo;
use crate::proxy::server_info::json::ServerInfoJson;
use crate::proxy::states::error::StateError;
use crate::proxy::states::*;
pub mod oneonestruct;
use buffer_plz::Cursor;
use bytes::BytesMut;
use oneonestruct::*;
use tokio::io::{
    AsyncReadExt, AsyncWriteExt, BufReader, copy_bidirectional_with_sizes
};
use tracing::trace;
mod error;
use std::fmt::Debug;
use std::io::Error;
use std::path::PathBuf;

use error::HandleOneOneError;
use tracing::error;

use super::handle_websocket;

const XATTR_HOST: &str = "user.host";
const XATTR_HTTP: &str = "user.http";
const XATTR_SNI: &str = "user.sni";

// client type alias
type OneOneRequest<T, E> = OneOneStruct<T, E, OneRequestLine>;
type ClientState<T, E> = ProxyState<OneOneRequest<T, E>>;

// server type alias
type OneOneResponse<T, E> = OneOneStruct<T, E, OneResponseLine>;
type ServerState<T, E> = ProxyState<OneOneResponse<T, E>>;

pub async fn handle_http<T, E, U>(
    mut client_state: ClientState<T, E>,
) -> Result<ConnectionState<U>, StateError>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin + Sync + Send + 'static + Debug,
    E: AsyncReadExt + AsyncWriteExt + Unpin + Sync + Send + 'static + Debug,
    OneOneRequest<T, E>: Reconnect,
    ConnectionState<U>: From<OneOneRequest<T, E>>,
{
    // Try twice to reconnect
    for _ in 0..2 {
        // 1. handle_one_one
        match handle_one_one(client_state).await {
            Ok(proxy_state) => {
                // 2. handle ws
                if let ProxyState::SwitchProtocol(mut conn, _) = proxy_state {
                    let req = CommanderRequest::ShouldProxyWs(conn.id);
                    conn.commander_sendr.send(req).await?;
                    let resp = conn
                        .commander_recvr
                        .recv()
                        .await
                        .ok_or(StateError::CommanderRecv("ShouldProxyWs"))?;
                    let result: bool = resp.try_into()?;
                    if result {
                        trace!("proxy ws| Y");
                        handle_websocket(conn).await?;
                    } else {
                        trace!("proxy ws| N");
                        let _ = copy_bidirectional_with_sizes(
                            &mut conn.reader,
                            &mut conn.writer,
                            CAPACITY_2MB,
                            CAPACITY_2MB,
                        )
                        .await;
                    }
                }
                break;
            }
            Err(e) => {
                client_state = match e {
                    /* Description:
                     *      Send to server failed
                     *
                     * Steps:
                     *      1. Reconnect to server
                     *      2. Set client_state to ProxyState::Send
                     */
                    HandleOneOneError::SendToServer(mut conn, e) => {
                        trace!("send_to_server err| {}", e);
                        conn.reconnect().await?;
                        ProxyState::Send(conn)
                    }

                    /* Description:
                     *      Read from server failed
                     *
                     * Steps:
                     *      1. Conver OneOneResponse to OneOneRequest
                     *          [ From trait implementation in
                     *          convert/response_to_request ]
                     *      2. Reconnect to server
                     *      3. Set client_state to ProxyState::ReadModFile with
                     *         a blank request specific ResumeInfo to satisfy
                     *         state transition requirements.
                     *         [ ResumeInfo::request() ]
                     */
                    HandleOneOneError::ReadFromServer(conn, e) => {
                        trace!("read_from_server err| {}", e);
                        let mut client =
                            OneOneStruct::<T, E, OneRequestLine>::from(conn);
                        client.reconnect().await?;
                        trace!("reconnected");
                        ProxyState::ReadModFile(client, ResumeInfo::request())
                    }

                    /* Description:
                     *      User specified a new server to connect to from
                     *      interceptor ui.
                     *
                     * Steps:
                     *      1. Convert AddReqInfo to ServerInfo [ TryFrom ]
                     *
                     *      2. Check if can be connected to the new address
                     *         i.e. if server conn is tls and new addr is also
                     *         tls. similarily for tcp.
                     *
                     *      3. If can reconnect, set serverinfo for
                     *         OneOneRequest and reconnect.
                     *
                     *      4. If cannot be connected, then convert
                     *         OneOneRequest to ConnnectionState [ From trait
                     *         in convert/to_state_connection ]
                     *
                     *          a. if,
                     *                  client      = tls://
                     *                  server      = tls://
                     *                  new server  = tcp://
                     *              then,
                     *                  ConnectionState::EstablishTlsTcp
                     *
                     *          b. if,
                     *                  client      = tcp://
                     *                  server      = tcp://
                     *                  new server  = tls://
                     *              then,
                     *                  ConnectionState::EstablishTcpTls
                     */
                    HandleOneOneError::NeedNewConnection(
                        mut conn,
                        addinfo,
                    ) => {
                        let server_info = ServerInfo::try_from(addinfo)?;
                        let can_reconnect = conn.can_reconnect(&server_info);
                        conn.set_server_info(server_info);
                        if can_reconnect {
                            trace!("reconnect| Y");
                            conn.reconnect().await?;
                            ProxyState::Send(conn)
                        } else {
                            trace!("reconnect| N");
                            let conn_state = ConnectionState::<U>::from(conn);
                            return Ok(conn_state);
                        }
                    }
                    HandleOneOneError::ProxyError(proxy_state_error) => {
                        return Err(StateError::Handler(proxy_state_error));
                    }
                    HandleOneOneError::StatusCode(status_code_error) => {
                        error!("{}", status_code_error);
                        break;
                    }
                    HandleOneOneError::InfoLine(info_line_error) => {
                        error!("{}", info_line_error);
                        break;
                    }
                };
                continue;
            }
        }
    }
    Ok(ConnectionState::End)
}

/* Description:
*       Function to handle a http/1.1 connection cycle.
*
* Args:
*       client_state: ProxyState<OneOneHandler<T, E, Request>>
*
* Trait Bound:
*       - T,E: AsyncReadExt + AsyncWriteExt + Unpin
*
*       - OneOneRequest<T, E>:  ReadWrite
*                               + Reconnect
*                               + Into<OneOneResponse<E, T>>
*                               + ReadWrite<State = ClientState<T, E>>
*
*       - OneOneResponse<T, E>: ReadWrite,
*
* Steps:
*      1. Run client_state until it ends.
*
*      2. Convert ClientState to OneOneRequest
*         [ TryFrom trait in convert/try_from_proxy_state ]
*
*           ProxyState<OneOneRequest> -> OneOneRequest
*
*          a. If client_state is NeedNewConnection
*              return HandleOneOneError::NeedNewConnection
*
*          b. If client_state is ServerClose
*              return HandleOneOneError::SendToServer
*
*      3. Check If client is logged,
*
*      4. If client is logged, set the extended attributes to the request file.
*
*      5. Convert OneOneRequest to OneOneResponse [ From trait in
*         convert/request_to_response ] and create new server_state,
*         ProxyState::Receive.
*
*      6. Run server_state until it ends.
*
*      7. Convert ServerState to OneOneResponse [ TryFrom trait in
*         convert/try_from_proxy_state ]
*
*               ProxyState<OneOneResponse> -> OneOneResponse
*
*           a. If server_state is ServerClose
*               return HandleOneOneError::ReadFromServer
*
*      8. Get the status code from server_state.
*
*      9. If the status code is 101, return ProxyState::SwitchProtocol(Ws)
*
*      10. else, return ProxyState::End
*
*      11. Else if client not not logged, Relay
*
* Returns:
*      Ok(ProxyState<OneOneHandler<E, T, Response>>)
*
* Error:
*      HandleOneOneError::ProxyError            [1] [5]
*      HandleOneOneError::SendToServer          [2]
*      HandleOneOneError::NeedNewConnection     [2]
*      HandleOneOneError::ReadFromServer        [7]
*      HandleOneOneError::StatusCode            [8]
*      HandleOneOneError::Relay                 [11]
*/

pub async fn handle_one_one<T, E>(
    mut client_state: ClientState<T, E>,
) -> Result<ServerState<E, T>, HandleOneOneError<T, E>>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
    OneOneRequest<T, E>: ReadWrite
        + Reconnect
        + Into<OneOneResponse<E, T>>
        + ReadWrite<State = ClientState<T, E>>,
    OneOneResponse<T, E>: ReadWrite,
{
    client_state = async_run(client_state).await?;
    let mut client_conn = OneOneRequest::<T, E>::try_from(client_state)?;

    match client_conn.path.is_some() {
        // 3. client logged
        true => {
            // set xttr
            let info = ServerInfoJson::from(&client_conn.server_info);
            if let Err(e) = set_attr(client_conn.path(), info) {
                error!("Set Attr| {}", e);
            }

            let mut server_state = ProxyState::Receive(client_conn.into());
            server_state = async_run(server_state).await?;
            let mut server_conn =
                OneOneResponse::<E, T>::try_from(server_state)?;

            let scode = OneResponseLine::try_build_infoline(
                server_conn.payload.take().unwrap(), // safe to unwrap
            )?
            .status()?;

            if scode == 101 {
                trace!("ws switch");
                return Ok(ProxyState::SwitchProtocol(
                    server_conn,
                    Protocol::WebSocket,
                ));
            }
            trace!("end");
            Ok(ProxyState::End(server_conn))
        }
        // 10. if client not logged, Relay
        false => {
            trace!("relay");
            let mut reader = BufReader::new(&mut client_conn.writer);
            let _ = tokio::io::copy_buf(&mut reader, &mut client_conn.reader)
                .await;
            Ok(ProxyState::End(client_conn.into()))
        }
    }
}

pub async fn read_http<T, U>(
    reader: &mut T,
    buf: &mut BytesMut,
) -> Result<OneOne<U>, OneOneRWError>
where
    T: AsyncReadExt + Unpin,
    U: OneInfoLine + std::fmt::Debug,
    OneMessageHead<U>: ParseBodyHeaders,
    OneOneRWError: From<oneone_plz::error::Error<U>>,
{
    let mut frame_state = State::<U>::new();
    let mut cbuf = Cursor::new(buf);
    loop {
        let event = fill_buffer(reader, &mut cbuf)
            .await
            .map_err(OneOneRWError::Read)?;
        frame_state = frame_state.try_next(event)?;
        if frame_state.is_ended() {
            return Ok(frame_state.try_into_frame()?);
        }
    }
}

fn set_attr(path: &PathBuf, info: ServerInfoJson) -> Result<(), Error> {
    xattr::set(path, XATTR_HOST, info.host.as_bytes())?;
    if info.http.is_some() {
        xattr::set(path, XATTR_HTTP, b"1")?;
    }
    if let Some(sni) = info.sni {
        xattr::set(path, XATTR_SNI, sni.as_bytes())?;
    }
    trace!("attr set");
    Ok(())
}
