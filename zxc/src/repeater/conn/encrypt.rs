use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use crate::repeater::prelude::*;

/* Description:
 *      convert RepeaterConn<TcpStream> -> RepeaterConn<TlsStream<TcpStream>>
 *
 * Steps:
 *      1. Get SNI from ServerInfo
 *      2. Encrypt connection with TlsConnector
 *
 * Error:
 *      RepeaterError::Encrypt
 */

impl RepeaterConn<TcpStream> {
    pub async fn encrypt(
        self,
        connector: Arc<TlsConnector>,
    ) -> Result<RepeaterConn<TlsStream<TcpStream>>, RepeaterError> {
        let sni = self.server_info.sni().to_owned();
        let stream = connector
            .connect(sni, self.stream)
            .await
            .map_err(RepeaterError::Encrypt)?;
        Ok(RepeaterConn {
            server_info: self.server_info,
            path: self.path,
            stream,
            update: self.update,
        })
    }
}
