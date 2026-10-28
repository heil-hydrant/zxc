use tokio::sync::mpsc::Sender;
use tracing::trace;

use super::WsComm;
use crate::commander::{CommanderResponse, CommunicateError};
use crate::id::{Id, SoldierId};
use crate::proxy::Role;

const WS_SEND_LOG: &str = "Ws Send Log";
const WS_SHOULD_INTERCEPT: &str = "Ws Should Intercept Response";
const WS_SET_NEED_RESPONSE: &str = "Ws Set Need Response";

// Stores Vector of WsComm
#[derive(Default)]
pub struct WsCommStorage {
    store: Vec<WsComm>,
}

impl WsCommStorage {
    pub fn add_handle(
        &mut self,
        id: SoldierId,
        client: Sender<CommanderResponse>,
        server: Sender<CommanderResponse>,
    ) {
        self.store
            .push(WsComm::new(id, client, server));
        trace!("ws comm registered");
    }

    pub async fn send_log(
        &mut self,
        id: SoldierId,
        role: Role,
    ) -> Result<(), CommunicateError> {
        let wscomm = self
            .store
            .iter_mut()
            .find(|x| x.id() == id)
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
            .iter_mut()
            .find(|x| x.id() == id)
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
            .iter_mut()
            .find(|x| x.id() == id)
            .ok_or(CommunicateError::NoId(id, WS_SET_NEED_RESPONSE))?;
        wscomm.set_need_response();
        Ok(())
    }

    pub fn store_as_mut(&mut self) -> &mut Vec<WsComm> {
        &mut self.store
    }

    pub fn iter(
        &self,
        role: Role,
    ) -> impl Iterator<Item = (SoldierId, &Sender<CommanderResponse>)> {
        self.store.iter().map(move |x| {
            (
                x.id(),
                match role {
                    Role::Client => x.client_sender(),
                    Role::Server => x.server_sender(),
                },
            )
        })
    }
}
