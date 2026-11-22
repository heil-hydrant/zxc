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
use crate::id::SoldierId;
use crate::proxy::Role;

#[derive(Default)]
pub struct Soldiers {
    pub http1: HttpCommStorage,
    pub http2: Http2CommStorage,
    pub ws: WsCommStorage,
}

impl Soldiers {
    fn get_sender(
        &self,
        id: SoldierId,
        ft: &FileType,
    ) -> Option<&Sender<CommanderResponse>> {
        match ft {
            FileType::Req | FileType::Res => match id {
                SoldierId::Connection(conn_id) => self.http1.get(conn_id),
                SoldierId::H2Stream {
                    conn_id,
                    stream_id,
                } => self
                    .http2
                    .get_stream(conn_id, stream_id),
            },
            FileType::Wreq => self.ws.get_sender(&id, Role::Server),
            FileType::Wres => self.ws.get_sender(&id, Role::Client),
        }
    }

    async fn broadcast_none(
        &self,
        map: &mut FxHashMap<usize, SoldierId>,
        ft: FileType,
    ) {
        for (_, soldier_id) in map.drain() {
            if let Some(sender) = self.get_sender(soldier_id, &ft) {
                let _ = sender
                    .send(CommanderResponse::Resume(None))
                    .await;
            }
        }
    }

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
        match self.get_sender(id, ft) {
            Some(sender) => Ok(sender.send(res).await?),
            None => Err(CommunicateError::ResponseNoId(id, res)),
        }
    }

    pub fn remove_from_http_store(&mut self, id: SoldierId) -> bool {
        match id {
            SoldierId::Connection(conn_id) => {
                self.http1.remove(conn_id)
                    | self
                        .http2
                        .remove_connection(conn_id)
                        .is_some()
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
        self.ws.remove(&id)
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
    ) {
        self.broadcast_none(map, FileType::Req)
            .await;
    }

    pub async fn broadcast_none_wreq(
        &mut self,
        map: &mut FxHashMap<usize, SoldierId>,
    ) {
        self.broadcast_none(map, FileType::Wreq)
            .await;
    }

    pub async fn broadcast_none_wres(
        &mut self,
        map: &mut FxHashMap<usize, SoldierId>,
    ) {
        self.broadcast_none(map, FileType::Wres)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn broadcast_none_drains_queue_when_receiver_is_gone() {
        let mut soldiers = Soldiers::default();
        drop(soldiers.add_http_handle(1));

        let mut queue = FxHashMap::default();
        queue.insert(7, SoldierId::Connection(1));

        soldiers
            .broadcast_none_http(&mut queue)
            .await;

        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_send_response_ft_http1() {
        let mut soldiers = Soldiers::default();
        let mut rx = soldiers.add_http_handle(1);

        soldiers
            .send_response_ft(
                SoldierId::Connection(1),
                &FileType::Req,
                CommanderResponse::Resume(None),
            )
            .await
            .expect("should send response");

        match rx.recv().await {
            Some(CommanderResponse::Resume(None)) => {}
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_send_response_ft_h2() {
        let mut soldiers = Soldiers::default();
        let mut rx = soldiers.add_h2_stream_handle(1, 3);

        soldiers
            .send_response_ft(
                SoldierId::H2Stream {
                    conn_id: 1,
                    stream_id: 3,
                },
                &FileType::Res,
                CommanderResponse::Resume(None),
            )
            .await
            .expect("should send response");

        match rx.recv().await {
            Some(CommanderResponse::Resume(None)) => {}
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_send_response_ft_ws() {
        let mut soldiers = Soldiers::default();
        let (tx_client, mut rx_client) = tokio::sync::mpsc::channel(1);
        let (tx_server, mut rx_server) = tokio::sync::mpsc::channel(1);
        let ws_id = SoldierId::Connection(10);

        soldiers.add_ws_handle(ws_id, tx_client, tx_server);

        soldiers
            .send_response_ft(
                ws_id,
                &FileType::Wreq,
                CommanderResponse::Resume(None),
            )
            .await
            .expect("should send wreq response");

        match rx_server.recv().await {
            Some(CommanderResponse::Resume(None)) => {}
            other => panic!("unexpected server response: {:?}", other),
        }

        soldiers
            .send_response_ft(
                ws_id,
                &FileType::Wres,
                CommanderResponse::Resume(None),
            )
            .await
            .expect("should send wres response");

        match rx_client.recv().await {
            Some(CommanderResponse::Resume(None)) => {}
            other => panic!("unexpected client response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_send_response_ft_no_id() {
        let mut soldiers = Soldiers::default();
        let result = soldiers
            .send_response_ft(
                SoldierId::Connection(999),
                &FileType::Req,
                CommanderResponse::Resume(None),
            )
            .await;

        match result {
            Err(CommunicateError::ResponseNoId(id, _)) => {
                assert_eq!(id, SoldierId::Connection(999));
            }
            other => panic!("expected ResponseNoId error, got: {:?}", other),
        }
    }
}
