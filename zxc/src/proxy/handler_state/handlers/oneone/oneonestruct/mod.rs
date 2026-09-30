use header_plz::{OneRequestLine, OneResponseLine};
use std::fmt::{Debug, Display};
use std::path::PathBuf;

use bytes::BytesMut;
use header_plz::OneInfoLine;
use http_plz::OneOne;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{Receiver, Sender};
use zxc_derive::{CanCommunicate, FileOps, GetRole, Id};

use crate::commander::{CommanderRequest, CommanderResponse};
use crate::history::message::from_commander::CommanderToHistory;
use crate::id::Id;
use crate::proxy::handler_state::role::{GetRole, Role};
use crate::proxy::handler_state::transition::can_communicate::CanCommunicate;
use crate::proxy::server_info::ServerInfo;
use crate::proxy::server_info::address::Address;
use crate::proxy::server_info::scheme::Scheme;
mod convert;
mod impl_add_raw;
mod impl_bytes_to_frame;
mod impl_can_log;
mod impl_drop_msg;
mod impl_frame_to_payload;
mod impl_get_history;
mod impl_intercept;
mod impl_log;
mod impl_read_write;
mod impl_reconnect;
mod impl_rewrite;
mod impl_send_history;
mod impl_should_intercept;
mod impl_should_log;
mod impl_should_rewrite;
mod impl_update_log_extension;
mod impl_update_resume_info;
pub use impl_read_write::OneOneRWError;

use crate::proxy::handler_state::transition::write_log::file_ops::FileOps;

#[derive(FileOps, GetRole, CanCommunicate, Id)]
pub struct OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    pub server_info: ServerInfo,
    pub buf: BytesMut,
    pub commander_sendr: Sender<CommanderRequest>,
    pub commander_recvr: Receiver<CommanderResponse>,
    pub payload: Option<BytesMut>,
    pub file: Option<File>,
    pub frame: Option<OneOne<U>>,
    pub id: usize,
    pub log_id: usize,
    pub path: Option<PathBuf>,
    pub reader: T,
    pub writer: E,
    history_sendr: Option<Sender<CommanderToHistory>>,
    role: Role,
    need_response: bool,
}

impl<T, E, U> OneOneStruct<T, E, U>
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

// Display trait for OneOneHandler
impl<T, E, U> Display for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OneOneStruct display")
    }
}

// Debug trait for OneOneHandler
impl<T, E, U> Debug for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OneOneStruct debug")
    }
}
