use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tracing::trace;
use zxc_derive::Id;

use crate::commander::CommanderResponse;
use crate::id::{Id, SoldierId};
use crate::proxy::Role;

// Structure that contains ws communication info
#[derive(Id)]
pub struct WsComm {
    id: SoldierId,
    client: Sender<CommanderResponse>,
    server: Sender<CommanderResponse>,
    log_index: usize,
    need_response: bool,
}

impl WsComm {
    pub fn new(
        id: SoldierId,
        client: Sender<CommanderResponse>,
        server: Sender<CommanderResponse>,
    ) -> Self {
        Self {
            id,
            client,
            server,
            log_index: 0,
            need_response: false,
        }
    }

    // increments the log_index
    pub fn inc_index(&mut self) {
        self.log_index += 1;
    }

    pub async fn send_log_response(
        &self,
        role: Role,
    ) -> Result<(), SendError<CommanderResponse>> {
        let resp = CommanderResponse::WsLog(self.log_index);
        trace!("ws log sent| {}| {}", role, self.log_index);
        let writer = match role {
            Role::Client => &self.client,
            Role::Server => &self.server,
        };
        writer.send(resp).await
    }

    pub async fn send_should_intercept_response(
        &mut self,
        stat: bool,
    ) -> Result<(), SendError<CommanderResponse>> {
        self.client
            .send(CommanderResponse::WsInterceptReply(stat))
            .await
    }

    pub fn set_need_response(&mut self) {
        self.need_response = true;
    }

    pub fn need_response(&self) -> bool {
        self.need_response
    }

    pub fn reset_need_response(&mut self) {
        self.need_response = false;
    }

    pub fn client_sender(&self) -> &Sender<CommanderResponse> {
        &self.client
    }

    pub fn server_sender(&self) -> &Sender<CommanderResponse> {
        &self.server
    }
}
