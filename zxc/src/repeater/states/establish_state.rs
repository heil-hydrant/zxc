use std::fmt::Display;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tracing::trace;

use crate::async_step::AsyncStep;
use crate::io::socket::establish_connection;
use crate::proxy::states::ZStream;
use crate::repeater::conn::RepeaterConn;
use crate::repeater::error::RepeaterError;

// Initial Repeater States, upto connection establishment + encryption
#[allow(clippy::large_enum_variant)]
pub enum RepeaterConnState {
    EstablishServerConn(RepeaterConn<ZStream>),
    NeedConnector(RepeaterConn<TcpStream>),
    EncryptConnection(RepeaterConn<TcpStream>, Arc<TlsConnector>),
    HandleTls(RepeaterConn<TlsStream<TcpStream>>),
    HandleTcp(RepeaterConn<TcpStream>),
}

// AsyncStep trait implementation for RepeaterConnState
impl AsyncStep for RepeaterConnState {
    type Error = RepeaterError;

    async fn next(self) -> Result<Self, Self::Error> {
        match self {
            /* Description:
             *      Establish connection with server
             *
             * Transition:
             *      if Tls -> NeedConnector
             *      if Tcp -> HandleTcp
             *
             * Error:
             *      RepeaterError::Connection
             */
            Self::EstablishServerConn(rconn) => {
                let stream = establish_connection(rconn.address()).await?;
                let rconn = RepeaterConn::<TcpStream>::from((rconn, stream));
                trace!("Y| {}", rconn.address());
                if rconn.tls() {
                    return Ok(Self::NeedConnector(rconn));
                }
                Ok(Self::HandleTcp(rconn))
            }

            /* Description:
             *      Encrypt connection
             *
             * Transitions:
             *      HandleTls
             *
             * Error:
             *      RepeaterError::Encrypt
             */
            Self::EncryptConnection(rconn, connector) => {
                let rconn = rconn.encrypt(connector).await?;
                trace!("Y");
                Ok(Self::HandleTls(rconn))
            }
            _ => Ok(self),
        }
    }

    fn is_ended(&self) -> bool {
        matches!(self, Self::HandleTls(_))
            || matches!(self, Self::HandleTcp(_))
            || matches!(self, Self::NeedConnector(_))
    }
}

impl Display for RepeaterConnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::EstablishServerConn(_) => "establish_server_conn",
            Self::EncryptConnection(_, _) => "encrypt_connection",
            _ => "",
        };
        write!(f, "{}| ", s)
    }
}
