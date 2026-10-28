use std::io::{Error, ErrorKind};

use bytes::BytesMut;
use tokio::net::UnixStream;

use crate::io::*;

pub fn read_from_unix(
    result: Result<(), Error>,
    sock: &mut UnixStream,
    buf: &mut BytesMut,
) -> Result<usize, UnixSockError> {
    if let Err(e) = result {
        if matches!(e.kind(), ErrorKind::WouldBlock) {
            return Err(UnixSockError::Block);
        } else {
            return Err(UnixSockError::Readable(e));
        }
    }
    match sock.try_read_buf(buf) {
        Ok(0) => Err(UnixSockError::EoF),
        Ok(n) => Ok(n),
        Err(e) => {
            if matches!(e.kind(), ErrorKind::WouldBlock) {
                Err(UnixSockError::Block)
            } else {
                Err(UnixSockError::Read(e))
            }
        }
    }
}
