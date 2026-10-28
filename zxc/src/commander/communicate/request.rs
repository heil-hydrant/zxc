use mime_plz::ContentType;
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::rustls::pki_types::CertificateDer;

use super::CommanderResponse;
use crate::commander::{Alpn, CertDigest};
use crate::id::SoldierId;
use crate::interceptor::InterToUI;
use crate::proxy::Role;

// Requests that soldiers sends to commander
pub enum CommanderRequest {
    ShouldProxy(
        usize,
        String,
        oneshot::Sender<Option<mpsc::Receiver<CommanderResponse>>>,
    ),
    // ----- Encryption -----
    // Client
    GetClientConnector(usize),
    // Server
    GetVerifier(usize),
    CheckServerConfigCache(usize, bool, CertDigest, Alpn),
    GetServerConfig(
        usize,
        bool,
        CertDigest,
        Vec<CertificateDer<'static>>,
        Alpn,
    ),

    // ----- Should Log -----
    // http
    ShouldLogHttp(SoldierId, String),
    ShouldLogHttpCt(SoldierId, ContentType),
    // ws
    WsLog(SoldierId, Role),

    // Intercept
    Intercept(SoldierId, InterToUI),
    // HTTP/2
    H2StreamRegister(
        SoldierId,
        oneshot::Sender<mpsc::Receiver<CommanderResponse>>,
    ),
    // WebSocket
    ShouldProxyWs(SoldierId),
    WsRegister(SoldierId),
    ShouldInterceptWsRespone(SoldierId),
    Close(SoldierId),
}
