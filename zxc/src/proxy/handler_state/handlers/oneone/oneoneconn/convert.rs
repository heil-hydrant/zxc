use header_plz::{OneRequestLine, OneResponseLine};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::Receiver;

use super::OneOneConn;
use crate::commander::CommanderResponse;
use crate::proxy::handler_state::additional_handler_info::AdditionalHandlerInfo;
use crate::proxy::handler_state::handlers::oneone::error::HandleOneOneError;
use crate::proxy::handler_state::role::Role;
use crate::proxy::prelude::*;
use crate::proxy::server_info::ServerInfo;
use crate::proxy::states::{
    ClientTlsStream, Connection, ConnectionState, ServerTlsStream, Tcp
};

//  Connection + Receiver<CommanderResponse> + ServerInfo =>
//          OneOneConn<OneRequestLine>

impl<T, E> From<(Connection<T, E>, Receiver<CommanderResponse>, ServerInfo)>
    for OneOneConn<T, E, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(
        (conn, commander_recvr, server_info): (
            Connection<T, E>,
            Receiver<CommanderResponse>,
            ServerInfo,
        ),
    ) -> Self {
        Self {
            server_info,
            buf: conn.buf,
            commander_sendr: conn.commander,
            commander_recvr,
            msg_bytes: None,
            file: None,
            msg: conn.request,
            id: SoldierId::Connection(conn.id),
            path: None,
            reader: conn.reader,
            role: Role::Server,
            need_response: false,
            writer: conn.writer,
            log_id: 0,
            history_sendr: None,
        }
    }
}

// Connection + AdditionalHandlerInfo => OneOneConn<Request>
//
// Used in:
//          State::SwitchTlsTcp
//          State::SwitchTcpTls

impl<T, E> From<(Connection<T, E>, AdditionalHandlerInfo)>
    for OneOneConn<T, E, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(
        (conn, addinfo): (Connection<T, E>, AdditionalHandlerInfo),
    ) -> Self {
        let mut one = OneOneConn::<T, E, OneRequestLine>::from((
            conn,
            addinfo.receiver,
            addinfo.server_info,
        ));
        one.log_id = addinfo.log_id;
        one.path = Some(addinfo.path);
        one.msg_bytes = Some(addinfo.payload);
        one.need_response = addinfo.should_intercept;
        one.history_sendr = addinfo.history_sender;
        one
    }
}

// OneOneConn<Request> to OneOneConn<Response>

impl<T, E> From<OneOneConn<T, E, OneRequestLine>>
    for OneOneConn<E, T, OneResponseLine>
{
    fn from(request: OneOneConn<T, E, OneRequestLine>) -> Self {
        Self {
            buf: request.buf,
            commander_sendr: request.commander_sendr,
            commander_recvr: request.commander_recvr,
            log_id: request.log_id,
            msg_bytes: None,
            file: request.file, // Can be reused in reconnect
            msg: None,
            id: request.id,
            path: request.path,
            reader: request.writer,
            role: Role::Client,
            need_response: request.need_response,
            writer: request.reader,
            server_info: request.server_info,
            history_sendr: request.history_sendr,
        }
    }
}

// OneOneConn<Response> => OneOneConn<Request>
//
// Used:
//      When Reading from server failed,
//      Since request file is already stored in Response struct reuse it.

impl<T, E> From<OneOneConn<T, E, OneResponseLine>>
    for OneOneConn<E, T, OneRequestLine>
{
    fn from(mut response: OneOneConn<T, E, OneResponseLine>) -> Self {
        response.buf.clear();
        Self {
            buf: response.buf,
            commander_sendr: response.commander_sendr,
            commander_recvr: response.commander_recvr,
            log_id: response.log_id,
            msg_bytes: None,
            file: response.file, // .req file reused in reconnect
            msg: None,
            id: response.id,
            path: response.path,
            reader: response.writer,
            role: Role::Server,
            need_response: response.need_response,
            writer: response.reader,
            server_info: response.server_info,
            history_sendr: response.history_sendr,
        }
    }
}

// OneOneConn<OneRequestLine> => Connection + AdditionalHandlerInfo
//
// Used:
//      ProxyState::NewConnection

impl<T, E> From<OneOneConn<T, E, OneRequestLine>>
    for (Connection<T, E>, AdditionalHandlerInfo)
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(
        oneone: OneOneConn<T, E, OneRequestLine>,
    ) -> (Connection<T, E>, AdditionalHandlerInfo) {
        let addinfo = AdditionalHandlerInfo::new(
            oneone.log_id,
            oneone.msg_bytes.unwrap(),
            oneone.path.unwrap(),
            oneone.need_response,
            oneone.commander_recvr,
            oneone.server_info,
            oneone.history_sendr,
        );
        let conn = Connection {
            buf: oneone.buf,
            commander: oneone.commander_sendr,
            request: oneone.msg,
            id: oneone.id.conn_id(),
            reader: oneone.reader,
            writer: oneone.writer,
        };
        (conn, addinfo)
    }
}

// OneOneConn<T,Tcp,OneRequestLine> => ConnectionState<T>
//
// Used in:
//      ProxyState::NewConnection
//      when,
//          client      = tcp://
//          server      = tcp://
//          new server  = tls://

impl<T> From<OneOneConn<T, Tcp, OneRequestLine>> for ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(oneone: OneOneConn<T, Tcp, OneRequestLine>) -> Self {
        let (conn, addinfo) = oneone.into();
        ConnectionState::EstablishTcpTls(conn, addinfo)
    }
}

// OneOneConn<ServerTlsStream<T>,ClientTlsStream<Tcp>,OneRequestLine> =>
//          ConnectionState<T>
//
// Used in:
//      ProxyState::NewConnection
//      when,
//          client      = tls://
//          server      = tls://
//          new server  = tcp://

impl<T>
    From<OneOneConn<ServerTlsStream<T>, ClientTlsStream<Tcp>, OneRequestLine>>
    for ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(
        oneone: OneOneConn<
            ServerTlsStream<T>,
            ClientTlsStream<Tcp>,
            OneRequestLine,
        >,
    ) -> Self {
        let (conn, addinfo) = oneone.into();
        ConnectionState::EstablishTlsTcp(conn, addinfo)
    }
}

// OneOneConn<ServerTlsStream<T>,Tcp,OneRequestLine> => ConnectionState<T>
//
//  Blank implementation
//
//  Should not reach this state, can_reconnect() should succeed in reconnect
//  state

impl<T> From<OneOneConn<ServerTlsStream<T>, Tcp, OneRequestLine>>
    for ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(_: OneOneConn<ServerTlsStream<T>, Tcp, OneRequestLine>) -> Self {
        unreachable!();
    }
}

// OneOneConn<T,ClientTlsStream<Tcp>,OneRequestLine> => ConnectionState<T>
//
//  Blank implementation
//
//  Should not reach this state, can_reconnect() should succeed in reconnect
//  state

impl<T> From<OneOneConn<T, ClientTlsStream<Tcp>, OneRequestLine>>
    for ConnectionState<T>
{
    fn from(_: OneOneConn<T, ClientTlsStream<Tcp>, OneRequestLine>) -> Self {
        unreachable!();
    }
}

// ProxyState<OneOneConn<T, E, OneRequestLine>> => OneOneConn<T,E,OneRequestLine>.
//
//      i.e. client_state to client_handler

impl<T, E> TryFrom<ProxyState<OneOneConn<T, E, OneRequestLine>>>
    for OneOneConn<T, E, OneRequestLine>
{
    type Error = HandleOneOneError<T, E>;

    fn try_from(
        client_state: ProxyState<OneOneConn<T, E, OneRequestLine>>,
    ) -> Result<Self, Self::Error> {
        match client_state {
            ProxyState::End(conn) => Ok(conn),
            ProxyState::ServerClose(conn, e) => {
                Err(HandleOneOneError::SendToServer(conn, e))
            }
            ProxyState::NewConnection(conn, addinfo) => {
                Err(HandleOneOneError::NeedNewConnection(conn, addinfo))
            }
            _ => unreachable!(),
        }
    }
}

impl<T, E> TryFrom<ProxyState<OneOneConn<T, E, OneResponseLine>>>
    for OneOneConn<T, E, OneResponseLine>
{
    type Error = HandleOneOneError<E, T>;

    fn try_from(
        server_state: ProxyState<OneOneConn<T, E, OneResponseLine>>,
    ) -> Result<Self, Self::Error> {
        match server_state {
            ProxyState::End(conn) => Ok(conn),
            ProxyState::ServerClose(conn, e) => {
                Err(HandleOneOneError::ReadFromServer(conn, e))
            }
            _ => unreachable!(),
        }
    }
}
