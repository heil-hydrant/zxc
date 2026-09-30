use tokio_rustls::rustls::server::Acceptor;
use tokio_rustls::{LazyConfigAcceptor, StartHandshake};

use super::*;
use crate::proxy::states::StateError;

/* Description:
 *      Performs client handshake. Called in ConnectionState::ClientHandShake.
 *
 * Generic:
 *      T: Client Stream = AsyncReadExt + AsyncWriteExt + Unpin
 *      E: Server Stream
 *
 * Steps:
 *      1. Create new Acceptor.
 *      2. Perform handshake by calling LazyConfigAcceptor::new() with
 *      acceptor and client stream.
 *
 * Error:
 *      StateError::ClientHandShake [2]
 */

impl<T, E> Connection<T, E>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin, // Client Stream
{
    pub async fn perform_handshake(
        self,
    ) -> Result<Connection<StartHandshake<T>, E>, StateError> {
        let acceptor = Acceptor::default();
        let lzc = LazyConfigAcceptor::new(acceptor, self.reader);
        let handshake = lzc
            .await
            .map_err(StateError::ClientHandshake)?;
        Ok(Connection {
            id: self.id,
            commander: self.commander,
            reader: handshake,
            writer: self.writer,
            buf: self.buf,
            request: self.request,
        })
    }
}
