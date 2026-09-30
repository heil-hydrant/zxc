use mime_plz::ContentType;
use openssl::hash::DigestBytes;
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::rustls::pki_types::CertificateDer;

use super::response::CommanderResponse;
use crate::interceptor::message::to_ui::InterToUI;
use crate::proxy::handler_state::role::Role;

// Requests that soldiers sends to commander
pub enum CommanderRequest {
    ShouldProxy(
        usize,
        String,
        oneshot::Sender<Option<mpsc::Receiver<CommanderResponse>>>,
    ),
    // ----- Encryption -----
    // Client
    GetClientConfig(usize),
    // Server
    GetVerifier(usize),
    CheckCertificate(usize, bool, DigestBytes),
    GenNewCert(usize, bool, DigestBytes, Vec<CertificateDer<'static>>),

    // ----- Should Log -----
    // http
    ShouldLogHttp(usize, String),
    ShouldLogHttpCt(usize, ContentType),
    // ws
    WsLog(usize, Role),

    // Intercept
    Intercept(usize, InterToUI),
    // WebSocket
    ShouldProxyWs(usize),
    WsRegister(usize),
    ShouldInterceptWsRespone(usize),
    Close(usize),
}
