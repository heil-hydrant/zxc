pub mod http;
pub mod http2;
pub mod ws;

use rustc_hash::FxHashMap;
use tokio::sync::mpsc::{Receiver, Sender};

use self::http::HttpCommStorage;
use self::http2::Http2CommStorage;
use self::ws::WsCommStorage;
use super::{CommanderResponse, CommunicateError, WS_REGISTER};
use crate::file_types::FileType;
use crate::id::{Id, SoldierId};
use crate::proxy::Role;

#[derive(Default)]
pub struct Soldiers {
    pub http1: HttpCommStorage,
    pub http2: Http2CommStorage,
    pub ws: WsCommStorage,
}

impl Soldiers {
    pub fn add_http_handle(
        &mut self,
        id: usize,
    ) -> Receiver<CommanderResponse> {
        self.http1.add_handle(id)
    }

    pub fn add_h2_stream_handle(
        &mut self,
        conn_id: usize,
        stream_id: u32,
    ) -> Receiver<CommanderResponse> {
        self.http2
            .add_stream_handle(conn_id, stream_id)
    }

    pub async fn send_ws_log(
        &mut self,
        id: SoldierId,
        role: Role,
    ) -> Result<(), CommunicateError> {
        self.ws.send_log(id, role).await
    }

    pub async fn ws_send_should_intercept_response(
        &mut self,
        id: SoldierId,
    ) -> Result<(), CommunicateError> {
        self.ws
            .send_should_intercept_response(id)
            .await
    }

    pub fn pop_http_sender(
        &mut self,
        id: usize,
    ) -> Result<Sender<CommanderResponse>, CommunicateError> {
        self.http1
            .pop_sender(id)
            .ok_or(CommunicateError::NoId(
                SoldierId::Connection(id),
                WS_REGISTER,
            ))
    }

    pub fn add_ws_handle(
        &mut self,
        id: SoldierId,
        client: Sender<CommanderResponse>,
        server: Sender<CommanderResponse>,
    ) {
        self.ws.add_handle(id, client, server);
    }

    pub async fn send_response_ft(
        &mut self,
        id: SoldierId,
        ft: &FileType,
        res: CommanderResponse,
    ) -> Result<(), CommunicateError> {
        match ft {
            FileType::Req | FileType::Res => match id {
                SoldierId::Connection(conn_id) => {
                    match self.http1.get(conn_id) {
                        Some(sender) => Ok(sender.send(res).await?),
                        None => Err(CommunicateError::ResponseNoId(id, res)),
                    }
                }
                SoldierId::H2Stream {
                    conn_id,
                    stream_id,
                } => {
                    match self
                        .http2
                        .get_stream(conn_id, stream_id)
                    {
                        Some(sender) => Ok(sender.send(res).await?),
                        None => Err(CommunicateError::ResponseNoId(id, res)),
                    }
                }
            },
            FileType::Wreq => {
                let iter = self.ws.iter(Role::Server);
                send_response(id, iter, res).await
            }
            FileType::Wres => {
                let iter = self.ws.iter(Role::Client);
                send_response(id, iter, res).await
            }
        }
    }

    pub fn remove_from_http_store(&mut self, id: SoldierId) -> bool {
        match id {
            SoldierId::Connection(conn_id) => {
                let removed_h1 = self.http1.remove(conn_id);
                let removed_h2 = self
                    .http2
                    .remove_connection(conn_id)
                    .is_some();
                removed_h1 || removed_h2
            }
            SoldierId::H2Stream {
                conn_id,
                stream_id,
            } => self
                .http2
                .remove_stream(conn_id, stream_id),
        }
    }

    pub fn remove_from_ws_store(&mut self, id: SoldierId) -> bool {
        remove_from_store(self.ws.store_as_mut(), id)
    }

    pub fn set_ws_need_response(
        &mut self,
        id: SoldierId,
    ) -> Result<(), CommunicateError> {
        self.ws.set_need_response(id)
    }

    pub async fn broadcast_none_http(
        &mut self,
        map: &mut FxHashMap<usize, SoldierId>,
    ) -> Result<(), CommunicateError> {
        for (&_log_id, &soldier_id) in map.iter() {
            let res = CommanderResponse::Resume(None);
            match soldier_id {
                SoldierId::Connection(conn_id) => {
                    if let Some(sender) = self.http1.get(conn_id) {
                        sender.send(res).await?;
                    }
                }
                SoldierId::H2Stream {
                    conn_id,
                    stream_id,
                } => {
                    if let Some(sender) = self
                        .http2
                        .get_stream(conn_id, stream_id)
                    {
                        sender.send(res).await?;
                    }
                }
            }
        }
        map.clear();
        Ok(())
    }

    pub async fn broadcast_none_wreq(
        &mut self,
        map: &mut FxHashMap<usize, SoldierId>,
    ) -> Result<(), CommunicateError> {
        for (&_log_id, &soldier_id) in map.iter() {
            if let Some((_, sender)) = self
                .ws
                .iter(Role::Server)
                .find(|(id, _)| *id == soldier_id)
            {
                sender
                    .send(CommanderResponse::Resume(None))
                    .await?;
            }
        }
        map.clear();
        Ok(())
    }

    pub async fn broadcast_none_wres(
        &mut self,
        map: &mut FxHashMap<usize, SoldierId>,
    ) -> Result<(), CommunicateError> {
        for (&_log_id, &soldier_id) in map.iter() {
            if let Some((_, sender)) = self
                .ws
                .iter(Role::Client)
                .find(|(id, _)| *id == soldier_id)
            {
                sender
                    .send(CommanderResponse::Resume(None))
                    .await?;
            }
        }
        map.clear();
        Ok(())
    }
}

pub fn remove_from_store<T>(list: &mut Vec<T>, id: SoldierId) -> bool
where
    T: Id,
{
    if let Some(index) = list
        .iter()
        .position(|comm| comm.id() == id)
    {
        list.swap_remove(index);
        return true;
    }
    false
}

pub async fn send_response<'a, T>(
    to_send: SoldierId,
    mut senders: T,
    response: CommanderResponse,
) -> Result<(), CommunicateError>
where
    T: Iterator<Item = (SoldierId, &'a Sender<CommanderResponse>)>,
{
    match senders.find(|(sid, _)| sid == &to_send) {
        Some((_, sender)) => Ok(sender.send(response).await?),
        None => Err(CommunicateError::ResponseNoId(to_send, response)),
    }
}
