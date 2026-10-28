#![allow(clippy::too_many_arguments)]

pub mod convert;
mod pipeline;

use std::path::PathBuf;

use bytes::BytesMut;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, Stream};
use tokio::fs::File;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{Error, Message};
use zxc_derive::{CanCommunicate, FileOps, GetRole, Id};

use crate::commander::{CommanderRequest, CommanderResponse};
use crate::history::CommanderToHistory;
use crate::proxy::prelude::*;

// Handler for ws connection
#[derive(FileOps, GetRole, CanCommunicate, Id)]
pub struct WsConn<T, E> {
    pub id: SoldierId,
    role: Role,
    reader: SplitStream<WebSocketStream<T>>,
    writer: SplitSink<WebSocketStream<E>, Message>,
    msg: Option<Message>,
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

impl<T, E> AsMut<BytesMut> for WsConn<T, E> {
    fn as_mut(&mut self) -> &mut BytesMut {
        &mut self.buf
    }
}

impl<T, E> WsConn<T, E>
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
        id: SoldierId,
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
            msg: None,
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
