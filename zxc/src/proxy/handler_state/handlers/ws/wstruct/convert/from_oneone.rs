use bytes::BytesMut;
use futures_util::StreamExt;
use header_plz::OneResponseLine;
use tokio::fs::create_dir;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol;
use tracing::trace;

use super::{WsStruct, *};
use crate::CAPACITY_2MB;
use crate::commander::CommanderRequest;
use crate::history::message::ws_register::HistoryWsRegisterInfo;
use crate::proxy::handler_state::handlers::oneonestruct::OneOneStruct;

/* Description:
 *      ToWs trait implementation for OneOneHandler
 *
 * Steps:
 *      1. Create websocket path and build
 *              old path = id/id.res
 *              new path = id/websocket
 *
 *      2. Register ws in Commander
 *          a. Build CommanderRequest::WsRegister with id
 *          b. Send it to Commander
 *          c. Receive WsRegisterReply and convert to
 *          (Receiver<CommanderResponse> , Sender<CommanderToHistory>)
 *          [ TryFrom trait in response/convert.rs ]
 *
 *      3. Register ws in History
 *          a. Build HistoryWsRegisterInfo with id, log_id, path and
 *          server_info
 *          b. Wrap in CommanderToHistory::RegisterWs
 *          c. Send it to History
 *
 *      4. Create WebSocketStream for
 *          a. client   => response.writer
 *          b. server   => response.reader
 *
 *      5. Split the WebSocketStream into read and write halves
 *
 *      6. Push "0" to path, so that when logging set_file_name() can be used
 *
 *      7. Build WsStruct for
 *          a. server => server_read_half , client_write_half
 *                      old_request_sender.clone(), old_response_recvr
 *                      old_buf
 *
 *          b. client => client_read_half , server_write_half
 *                      old_response_sender, WsRegisterReply.receiver
 *                      new_buf
 *
 *      8. Clone history_sender, path from commander response for server and client
 *
 * Error:
 *      WsCreationError
 *          CreateDir   [1]
 *          Send        [2.b]
 *          NoReply     [2.c]
 *          HistorySend [3]
 */

impl<T, E> ToWs<T, E> for OneOneStruct<T, E, OneResponseLine>
where
    T: AsyncRead + AsyncWrite + Unpin,
    E: AsyncRead + AsyncWrite + Unpin,
{
    async fn convert(mut self) -> Result<TupleWs<T, E>, WsCreationError> {
        let mut path = self.path.take().unwrap(); // safe to unwrap
        path.pop();
        path.push("websocket");
        create_dir(&path).await?; // 1
        trace!("ws dir| {}", path.display());

        // 2. Register WebSocket in Commander
        let msg = CommanderRequest::WsRegister(self.id);
        self.commander_sendr.send(msg).await?;
        let resp = self
            .commander_recvr
            .recv()
            .await
            .ok_or(WsCreationError::NoReply)?;
        let (from_commander, to_history) = resp.try_into()?;
        trace!("websocket registered");

        // 3. Register WebSocket in History
        let hreg = HistoryWsRegisterInfo::new(
            self.id,
            self.log_id,
            path.clone(),
            self.server_info,
        );
        let hres = CommanderToHistory::RegisterWs(hreg);
        to_history.send(hres).await?;

        // 4.a. Client Side
        let client_stream = WebSocketStream::from_raw_socket(
            self.writer,
            protocol::Role::Server,
            None,
        )
        .await;

        // 4.b. Server Side
        let server_stream = WebSocketStream::from_raw_socket(
            self.reader,
            protocol::Role::Client,
            None,
        )
        .await;

        // 5. Split
        let (client_write_half, client_read_half) = client_stream.split();
        let (server_write_half, server_read_half) = server_stream.split();

        path.push("0");

        // 7. Build WsStruct
        let server = WsStruct::new(
            self.buf,
            self.commander_recvr,
            self.commander_sendr.clone(),
            to_history.clone(),
            self.log_id,
            self.id,
            path.clone(),
            client_read_half,
            Role::Server,
            server_write_half,
        );
        let client = WsStruct::new(
            BytesMut::with_capacity(CAPACITY_2MB),
            from_commander,
            self.commander_sendr,
            to_history,
            self.log_id,
            self.id,
            path,
            server_read_half,
            Role::Client,
            client_write_half,
        );
        Ok((server, client))
    }
}
