use rustc_hash::FxHashMap;
use tokio::sync::mpsc::Sender;
use tracing::trace;

use super::WsComm;
use crate::commander::{CommanderResponse, CommunicateError};
use crate::id::SoldierId;
use crate::proxy::Role;

const WS_SEND_LOG: &str = "Ws Send Log";
const WS_SHOULD_INTERCEPT: &str = "Ws Should Intercept Response";
const WS_SET_NEED_RESPONSE: &str = "Ws Set Need Response";

// Stores map of WsComm by SoldierId
#[derive(Default)]
pub struct WsCommStorage {
    store: FxHashMap<SoldierId, WsComm>,
}

impl WsCommStorage {
    pub fn add_handle(
        &mut self,
        id: SoldierId,
        client: Sender<CommanderResponse>,
        server: Sender<CommanderResponse>,
    ) {
        self.store
            .insert(id, WsComm::new(id, client, server));
        trace!("ws comm registered");
    }

    pub async fn send_log(
        &mut self,
        id: SoldierId,
        role: Role,
    ) -> Result<(), CommunicateError> {
        let wscomm = self
            .store
            .get_mut(&id)
            .ok_or(CommunicateError::NoId(id, WS_SEND_LOG))?;
        wscomm.inc_index();
        wscomm
            .send_log_response(role)
            .await
            .map_err(Into::into)
    }

    pub async fn send_should_intercept_response(
        &mut self,
        id: SoldierId,
    ) -> Result<(), CommunicateError> {
        let wscomm = self
            .store
            .get_mut(&id)
            .ok_or(CommunicateError::NoId(id, WS_SHOULD_INTERCEPT))?;
        let tosend = wscomm.need_response();
        if tosend {
            wscomm.reset_need_response();
        }
        wscomm
            .send_should_intercept_response(tosend)
            .await
            .map_err(Into::into)
    }

    pub fn set_need_response(
        &mut self,
        id: SoldierId,
    ) -> Result<(), CommunicateError> {
        let wscomm = self
            .store
            .get_mut(&id)
            .ok_or(CommunicateError::NoId(id, WS_SET_NEED_RESPONSE))?;
        wscomm.set_need_response();
        Ok(())
    }

    pub fn remove(&mut self, id: &SoldierId) -> bool {
        self.store.remove(id).is_some()
    }

    pub fn get_sender(
        &self,
        id: &SoldierId,
        role: Role,
    ) -> Option<&Sender<CommanderResponse>> {
        self.store.get(id).map(|x| match role {
            Role::Client => x.client_sender(),
            Role::Server => x.server_sender(),
        })
    }
}
