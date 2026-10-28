use std::io::{Error, ErrorKind, IoSlice};
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::time::timeout;

use crate::io::*;

pub const DEFAULT_ID: &str = "[0,";
pub const CLOSE_SQUARE_BRACKET: &str = "]";
const WRITE_TIMEOUT: Duration = Duration::from_secs(1);

// https://github.com/rust-lang/rust/issues/70436
//
// https://doc.rust-lang.org/src/std/io/mod.rs.html#1753-1768

pub async fn unix_write_vec_and_flush(
    stream: &mut UnixStream,
    mut buf: &mut [IoSlice<'_>],
) -> Result<usize, Error> {
    //loop {
    //    stream.writable().await?;
    //    match stream.try_write_vectored(buf) {
    //        Ok(n) => {
    //            stream.flush().await?;
    //            return Ok(n);
    //        }
    //        Err(e) => {
    //            if matches!(e.kind(), ErrorKind::WouldBlock) {
    //                continue;
    //            }
    //            return Err(e);
    //        }
    //    }
    //}
    timeout(WRITE_TIMEOUT, async {
        while !buf.is_empty() {
            stream.writable().await?;
            match stream.try_write_vectored(buf) {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => IoSlice::advance_slices(&mut buf, n),
                Err(e) => {
                    if matches!(e.kind(), ErrorKind::WouldBlock) {
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        stream.flush().await?;
        Ok(0)
    })
    .await
    .map_err(|_| {
        Error::new(ErrorKind::TimedOut, "unix socket write timed out")
    })?
}

pub async fn build_slice_and_write(
    id: usize,
    data: String,
    stream: &mut UnixStream,
) -> Result<(), UnixSockError> {
    let id_string;
    let prefix = if id == 0 {
        DEFAULT_ID
    } else {
        id_string = format!("[{id},");
        &id_string
    };
    let mut slice = build_slice(prefix, &data);
    unix_write_vec_and_flush(stream, &mut slice)
        .await
        .map_err(UnixSockError::Write)?;
    Ok(())
}

fn build_slice<'a>(id: &'a str, data: &'a str) -> Vec<IoSlice<'a>> {
    vec![
        IoSlice::new(id.as_bytes()),
        IoSlice::new(data.as_bytes()),
        IoSlice::new(b"]"),
    ]
}
