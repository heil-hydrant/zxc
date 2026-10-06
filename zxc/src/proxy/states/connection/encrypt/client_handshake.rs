use tokio_rustls::rustls::server::Acceptor;
use tokio_rustls::{LazyConfigAcceptor, StartHandshake};

use super::*;
use crate::proxy::states::StateError;

impl<T, E> Connection<T, E>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin, // Client Stream
{
    pub async fn perform_handshake(
        self,
    ) -> Result<Connection<StartHandshake<T>, E>, StateError> {
        let handshake =
            LazyConfigAcceptor::new(Acceptor::default(), self.reader)
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
