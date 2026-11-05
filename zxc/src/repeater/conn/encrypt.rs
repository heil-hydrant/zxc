use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use crate::commander::{ALPN_H1, ALPN_H2};
use crate::repeater::prelude::*;

/* Description:
 *      convert RepeaterConn<TcpStream> -> RepeaterConn<TlsStream<TcpStream>>
 *
 * Steps:
 *      1. Get SNI from ServerInfo
 *      2. Restrict ALPN to the protocol being replayed
 *      3. Encrypt connection with TlsConnector
 *      4. Verify the negotiated ALPN
 *
 * Error:
 *      RepeaterError::Encrypt
 *      RepeaterError::UnexpectedAlpn
 */

impl RepeaterConn<TcpStream> {
    pub async fn encrypt(
        self,
        connector: Arc<TlsConnector>,
    ) -> Result<RepeaterConn<TlsStream<TcpStream>>, RepeaterError> {
        let is_h2 = self.is_h2;
        let mut config = connector.config().as_ref().clone();
        config.alpn_protocols = if is_h2 {
            vec![ALPN_H2.to_vec()]
        } else {
            vec![ALPN_H1.to_vec()]
        };
        let connector = TlsConnector::from(Arc::new(config));

        let sni = self.server_info.sni().to_owned();
        let stream = connector
            .connect(sni, self.stream)
            .await
            .map_err(RepeaterError::Encrypt)?;

        let negotiated_bytes = stream.get_ref().1.alpn_protocol();
        let valid = if is_h2 {
            negotiated_bytes == Some(ALPN_H2)
        } else {
            negotiated_bytes.is_none() || negotiated_bytes == Some(ALPN_H1)
        };
        if !valid {
            return Err(RepeaterError::UnexpectedAlpn {
                expected: if is_h2 {
                    "h2"
                } else {
                    "http/1.1"
                },
                negotiated: negotiated_bytes
                    .map(|alpn| String::from_utf8_lossy(alpn).into_owned()),
            });
        }

        Ok(RepeaterConn {
            server_info: self.server_info,
            path: self.path,
            stream,
            update: self.update,
            is_h2,
            h2_info: self.h2_info,
        })
    }
}
