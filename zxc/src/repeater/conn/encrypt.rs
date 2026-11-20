use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use crate::commander::{ALPN_H1, ALPN_H2};
use crate::repeater::prelude::*;

// Converts `RepeaterConn<TcpStream>` into
// `RepeaterConn<TlsStream<TcpStream>>`.
//
// # Steps
//
// 1. Get the SNI from `ServerInfo`.
// 2. Override ALPN for the protocol being replayed.
// 3. Encrypt the connection with `TlsConnector`.
// 4. Verify the negotiated ALPN.
//
// # Errors
//
// - `RepeaterError::Encrypt`
// - `RepeaterError::UnexpectedAlpn`

impl RepeaterConn<TcpStream> {
    pub async fn encrypt(
        self,
        connector: Arc<TlsConnector>,
    ) -> Result<RepeaterConn<TlsStream<TcpStream>>, RepeaterError> {
        let is_h2 = self.is_h2;
        let connector = connector.with_alpn(if is_h2 {
            vec![ALPN_H2.to_vec()]
        } else {
            vec![ALPN_H1.to_vec()]
        });

        let sni = self.server_info.sni_or_host()?;
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
