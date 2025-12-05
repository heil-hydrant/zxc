use buffer::{Cursor, Event};
use thiserror::Error;
use tokio::io::{
    AsyncReadExt, {self}
};
use tokio::net::TcpStream;
use tracing::trace;

use crate::proxy::server_info::address::Address;

// Socket related IO operations

#[derive(Debug, Error)]
#[error("{address}| {error}")]
pub struct ConnectError {
    address: String,
    error: io::Error,
}

/* Description:
 *      Given an address, establish a connection
 *
 * Args:
 *      address: &Address
 *
 * Steps:
 *      Get SocketAddr / DNS and connect
 *
 * Returns:
 *      Ok(TcpStream)
 *
 * Error:
 *      io::Error
 */

impl From<(&Address, io::Error)> for ConnectError {
    fn from((address, error): (&Address, io::Error)) -> Self {
        Self {
            address: address.to_string(),
            error,
        }
    }
}

pub async fn establish_connection(
    address: &Address,
) -> Result<TcpStream, ConnectError> {
    let result = match address {
        Address::Socket(socket_addr) => TcpStream::connect(socket_addr).await,
        Address::Dns(addr) => TcpStream::connect(addr).await,
    };
    result.map_err(|e| ConnectError::from((address, e)))
}

/* Description:
 *      Given a generic type that implements AsyncReadExt and a buffer
 *      (Cursor), read the buffer from the generic type.
 */

pub async fn fill_buffer<'a, 'b, T>(
    stream: &mut T,
    buf: &'a mut Cursor<'b>,
) -> Result<Event<'a, 'b>, io::Error>
where
    T: AsyncReadExt + Unpin,
{
    let size = stream.read_buf(buf.as_mut()).await?;
    if size == 0 {
        trace!("EOF");
        Ok(Event::End(buf))
    } else {
        trace!("read");
        Ok(Event::Read(buf))
    }
}
