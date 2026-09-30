use super::*;

/* Type changing struct
 * https://github.com/rust-lang/rust/issues/86555
 */

// Establish Server Connection = ZStream -> Tcp
// Complete Handshake = Tls
// TlsTcp = TlsTls -> TlsTcp
// TcpTls = TcpTcp -> TcpTls

impl<T, E, U> From<(Connection<T, E>, U)> for Connection<T, U> {
    fn from((conn, stream): (Connection<T, E>, U)) -> Self {
        Connection {
            writer: stream,
            buf: conn.buf,
            commander: conn.commander,
            request: conn.request,
            id: conn.id,
            reader: conn.reader,
        }
    }
}
