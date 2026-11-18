use bytes::BytesMut;
use futures_util::StreamExt;
use header_plz::OneResponseLine;
use tokio::fs::create_dir;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol;
use tracing::trace;

use super::WsConn;
use crate::CAPACITY_2MB;
use crate::commander::*;
use crate::history::*;
use crate::proxy::handler_state::handlers::oneoneconn::OneOneConn;
use crate::proxy::handler_state::handlers::ws::error::WsCreationError;
use crate::proxy::prelude::*;

// T: Client
// E: Server

pub type TupleWs<T, E> = (WsConn<E, T>, WsConn<T, E>);

// Trait to convert to TupleWs
pub trait ToWs<T, E> {
    async fn convert(self) -> Result<TupleWs<T, E>, WsCreationError>;
}

// Converts a logged HTTP/1.1 request into the WebSocket relay.
//
// # Steps
//
// 1. Build `id/websocket` from `id/id.res`.
// 2. Register the WebSocket with the commander.
//    a. Build `CommanderRequest::WsRegister` with the ID.
//    b. Send it and convert the reply into the response receiver and
//       history sender.
// 3. Register it with the history handler.
//    a. Build `HistoryWsRegisterInfo` with the ID, log ID, path, and
//       server info.
//    b. Wrap it in `CommanderToHistory::RegisterWs` and send it.
// 4. Create the client and server `WebSocketStream`s.
// 5. Split each stream into read and write halves.
// 6. Push `"0"` to the path so logging can use `set_file_name()`.
// 7. Build the server and client `WsConn`s.
// 8. Clone the history sender and path for both halves.
//
// # Errors
//
// - `WsCreationError::CreateDir` [1]
// - `WsCreationError::Send` [2.b]
// - `WsCreationError::NoReply` [2.c]
// - `WsCreationError::HistorySend` [3]

impl<T, E> ToWs<T, E> for OneOneConn<T, E, OneResponseLine>
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
        let server = WsConn::new(
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
        let client = WsConn::new(
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
