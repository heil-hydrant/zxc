mod pipeline;

use std::path::PathBuf;

use bytes::BytesMut;
use header_plz::{RequestLine, ResponseLine};
use http_plz::{Message, Request};
use tokio::fs::File;
use tokio::sync::mpsc::{Receiver, Sender};
use two_plz::client::{ResponseFuture, SendRequest};
use two_plz::server::SendResponse;
use zxc_derive::{CanCommunicate, FileOps, GetRole, Id, Log, SendHistory};

use crate::commander::{CommanderRequest, CommanderResponse};
use crate::history::CommanderToHistory;
use crate::proxy::prelude::*;

#[derive(GetRole, Id, CanCommunicate, FileOps, SendHistory, Log)]
pub struct TwoConn<T> {
    pub id: SoldierId,
    pub role: Role,
    // server
    pub msg: Option<Message<T>>,
    pub send_response: SendResponse,
    // client
    pub send_request: SendRequest,
    pub recv_response: Option<ResponseFuture>,
    // cmd
    pub commander_sendr: Sender<CommanderRequest>,
    pub commander_recvr: Receiver<CommanderResponse>,
    pub server_info: ServerInfo,
    // log
    pub history_sendr: Option<Sender<CommanderToHistory>>,
    pub path: Option<PathBuf>,
    pub file: Option<File>,
    pub log_id: usize,
    pub buf: BytesMut,
    pub msg_bytes: Option<BytesMut>,
    // intercept
    pub need_response: bool,
}

impl<T> AsMut<BytesMut> for TwoConn<T> {
    fn as_mut(&mut self) -> &mut BytesMut {
        &mut self.buf
    }
}

impl TwoConn<RequestLine> {
    pub fn new(
        id: SoldierId,
        msg: Request,
        send_response: SendResponse,
        send_request: SendRequest,
        commander_sendr: Sender<CommanderRequest>,
        commander_recvr: Receiver<CommanderResponse>,
        server_info: ServerInfo,
    ) -> Self {
        Self {
            id,
            role: Role::Server,
            msg: Some(msg),
            send_response,
            send_request,
            recv_response: None,
            commander_sendr,
            commander_recvr,
            server_info,
            history_sendr: None,
            path: None,
            file: None,
            log_id: 0,
            buf: BytesMut::new(),
            msg_bytes: None,
            need_response: false,
        }
    }
}

impl From<TwoConn<RequestLine>> for TwoConn<ResponseLine> {
    fn from(request: TwoConn<RequestLine>) -> Self {
        Self {
            id: request.id,
            role: Role::Client,
            msg: None,
            send_response: request.send_response,
            send_request: request.send_request,
            recv_response: request.recv_response,
            commander_sendr: request.commander_sendr,
            commander_recvr: request.commander_recvr,
            server_info: request.server_info,
            history_sendr: request.history_sendr,
            path: request.path,
            file: request.file,
            log_id: request.log_id,
            buf: request.buf,
            msg_bytes: None,
            need_response: request.need_response,
        }
    }
}

impl TryFrom<ProxyState<TwoConn<RequestLine>>> for TwoConn<RequestLine> {
    type Error = ProxyStateError;

    fn try_from(
        state: ProxyState<TwoConn<RequestLine>>,
    ) -> Result<Self, Self::Error> {
        match state {
            ProxyState::End(conn) => Ok(conn),
            _ => unreachable!(),
        }
    }
}

impl TryFrom<ProxyState<TwoConn<ResponseLine>>> for TwoConn<ResponseLine> {
    type Error = ProxyStateError;

    fn try_from(
        state: ProxyState<TwoConn<ResponseLine>>,
    ) -> Result<Self, Self::Error> {
        match state {
            ProxyState::End(conn) => Ok(conn),
            _ => unreachable!(),
        }
    }
}
