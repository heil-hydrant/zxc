use header_plz::{RequestLine, ResponseLine};

use super::TwoConn;
use crate::proxy::prelude::*;

impl From<TwoConn<RequestLine>> for TwoConn<ResponseLine> {
    fn from(request: TwoConn<RequestLine>) -> Self {
        Self {
            id: request.id,
            role: Role::Client,
            msg: None,
            send_response: request.send_response,
            send_request: request.send_request,
            recv_response: request.recv_response,
            commander_sendr: request.commander_sendr,
            commander_recvr: request.commander_recvr,
            server_info: request.server_info,
            history_sendr: request.history_sendr,
            path: request.path,
            file: request.file,
            log_id: request.log_id,
            buf: request.buf,
            msg_bytes: None,
            need_response: request.need_response,
            h2_pseudo: None,
        }
    }
}

impl TryFrom<ProxyState<TwoConn<RequestLine>>> for TwoConn<RequestLine> {
    type Error = ProxyStateError;

    fn try_from(
        state: ProxyState<TwoConn<RequestLine>>,
    ) -> Result<Self, Self::Error> {
        match state {
            ProxyState::End(conn) => Ok(conn),
            _ => unreachable!(),
        }
    }
}

impl TryFrom<ProxyState<TwoConn<ResponseLine>>> for TwoConn<ResponseLine> {
    type Error = ProxyStateError;

    fn try_from(
        state: ProxyState<TwoConn<ResponseLine>>,
    ) -> Result<Self, Self::Error> {
        match state {
            ProxyState::End(conn) => Ok(conn),
            _ => unreachable!(),
        }
    }
}
