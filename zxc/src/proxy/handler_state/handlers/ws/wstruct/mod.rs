#![allow(clippy::too_many_arguments)]
use std::path::PathBuf;

use bytes::BytesMut;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, Stream};
use tokio::fs::File;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{Error, Message};
use zxc_derive::{CanCommunicate, FileOps, GetRole, Id};

use crate::history::message::from_commander::CommanderToHistory;
use crate::id::Id;
use crate::proxy::handler_state::CanCommunicate;
pub mod convert;

mod impl_drop_msg;
use super::*;
use crate::commander::{CommanderRequest, CommanderResponse};
use crate::proxy::handler_state::role::{GetRole, Role};
mod impl_add_raw;
mod impl_bytes_to_frame;
mod impl_can_log;
mod impl_frame_to_payload;
mod impl_get_history;
mod impl_intercept;
mod impl_log;
mod impl_read_write;
mod impl_rewrite;
mod impl_send_history;
mod impl_should_intercept;
mod impl_should_log;
mod impl_should_rewrite;
mod impl_update_resume_info;
use crate::proxy::handler_state::transition::FileOps;

// Handler for ws connection
#[derive(FileOps, GetRole, CanCommunicate, Id)]
pub struct WsStruct<T, E> {
    id: usize,
    role: Role,
    reader: SplitStream<WebSocketStream<T>>,
    writer: SplitSink<WebSocketStream<E>, Message>,
    frame: Option<Message>,
    path: PathBuf,
    http_id: usize,
    log_id: usize,
    file: Option<File>,
    buf: BytesMut,
    // Communicate
    commander_sendr: Sender<CommanderRequest>,
    commander_recvr: Receiver<CommanderResponse>,
    history_sendr: Sender<CommanderToHistory>,
}

impl<T, E> WsStruct<T, E>
where
    SplitSink<WebSocketStream<E>, Message>: Sink<Message>,
    SplitStream<WebSocketStream<T>>: Stream<Item = Result<Message, Error>>,
{
    fn new(
        buf: BytesMut,
        commander_recvr: Receiver<CommanderResponse>,
        commander_sendr: Sender<CommanderRequest>,
        history: Sender<CommanderToHistory>,
        http_id: usize,
        id: usize,
        path: PathBuf,
        reader: SplitStream<WebSocketStream<T>>,
        role: Role,
        writer: SplitSink<WebSocketStream<E>, Message>,
    ) -> Self {
        Self {
            buf,
            commander_recvr,
            commander_sendr,
            file: None,
            frame: None,
            history_sendr: history,
            http_id,
            id,
            log_id: 0,
            path,
            reader,
            role,
            writer,
        }
    }
}
