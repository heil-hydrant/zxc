mod convert;
mod pipeline;

use std::fmt::{Debug, Display};
use std::path::PathBuf;

use bytes::BytesMut;
use header_plz::OneInfoLine;
use http_plz::OneOne;
pub use pipeline::OneOneRWError;
use tokio::fs::File;
use tokio::sync::mpsc::{Receiver, Sender};
use zxc_derive::{CanCommunicate, FileOps, GetRole, Id, Log, SendHistory};

use crate::commander::{CommanderRequest, CommanderResponse};
use crate::history::CommanderToHistory;
use crate::proxy::prelude::*;

#[derive(FileOps, GetRole, CanCommunicate, Id, SendHistory, Log)]
pub struct OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    pub server_info: ServerInfo,
    pub buf: BytesMut,
    pub commander_sendr: Sender<CommanderRequest>,
    pub commander_recvr: Receiver<CommanderResponse>,
    pub msg_bytes: Option<BytesMut>,
    pub file: Option<File>,
    pub msg: Option<OneOne<U>>,
    pub id: SoldierId,
    pub log_id: usize,
    pub path: Option<PathBuf>,
    pub reader: T,
    pub writer: E,
    pub history_sendr: Option<Sender<CommanderToHistory>>,
    pub role: Role,
    pub need_response: bool,
}

impl<T, E, U> AsMut<BytesMut> for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    fn as_mut(&mut self) -> &mut BytesMut {
        &mut self.buf
    }
}

impl<T, E, U> OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    pub fn scheme(&self) -> Scheme {
        self.server_info.scheme()
    }

    pub fn address(&self) -> &Address {
        self.server_info.address()
    }

    pub fn set_server_info(&mut self, server_info: ServerInfo) {
        self.server_info = server_info;
    }
}

impl<T, E, U> Display for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OneOneStruct display")
    }
}

impl<T, E, U> Debug for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OneOneStruct debug")
    }
}
