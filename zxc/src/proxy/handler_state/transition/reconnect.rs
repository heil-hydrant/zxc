use crate::proxy::prelude::*;

/// Reconnects a server stream after interception.
///
/// HTTP/1.1 only. Implemented by `OneOneConn`.
pub trait Reconnect {
    /// Returns whether the stream can target `server_info`.
    ///
    /// `false` when the user changed the scheme, host, or SNI during
    /// interception.
    fn can_reconnect(&self, server_info: &ServerInfo) -> bool;

    /// Drops the old transport and dials `server_info`.
    ///
    /// # Errors
    ///
    /// Returns [`ProxyStateError`] when the TCP connect or TLS handshake
    /// fails.
    async fn reconnect(&mut self) -> Result<(), ProxyStateError>;
}
