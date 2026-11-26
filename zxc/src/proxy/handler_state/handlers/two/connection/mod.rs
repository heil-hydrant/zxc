use std::path::PathBuf;

use bytes::BytesMut;
use header_plz::RequestLine;
use http_plz::{Message, Request};
use tokio::fs::File;
use tokio::sync::mpsc::{Receiver, Sender};
use two_plz::client::{ResponseFuture, SendRequest};
use two_plz::server::SendResponse;
use zxc_derive::{AttachFile, CanCommunicate, GetRole, Id, Log, SendHistory};

use crate::commander::{CommanderRequest, CommanderResponse};
use crate::history::CommanderToHistory;
use crate::interceptor::H2Info;
use crate::proxy::prelude::*;

mod convert;
mod pipeline;

#[derive(GetRole, Id, CanCommunicate, AttachFile, SendHistory, Log)]
pub struct TwoConn<T> {
    pub(crate) id: SoldierId,
    pub(crate) role: Role,
    // server
    pub(crate) msg: Option<Message<T>>,
    pub(crate) send_response: SendResponse,
    // client
    pub(crate) send_request: SendRequest,
    pub(crate) recv_response: Option<ResponseFuture>,
    // cmd
    pub(crate) commander_sendr: Sender<CommanderRequest>,
    pub(crate) commander_recvr: Receiver<CommanderResponse>,
    pub(crate) server_info: ServerInfo,
    // log
    pub(crate) history_sendr: Option<Sender<CommanderToHistory>>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) file: Option<File>,
    pub(crate) log_id: usize,
    pub(crate) buf: BytesMut,
    pub(crate) msg_bytes: Option<BytesMut>,
    // intercept
    pub(crate) need_response: bool,
    pub(crate) h2_pseudo: Option<H2Info>,
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
            h2_pseudo: None,
        }
    }
}
