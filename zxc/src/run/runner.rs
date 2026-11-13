use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio::select;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, trace};

use super::boundary::*;
use crate::io::{
    UnixSockError, build_slice_and_write, read_from_unix, split_json_frame, write_and_flush
};

const CLOSE_MSG: &[u8] = br#"[0,{"Action": "Close"}]"#;

/* Steps:
 * ----- loop -----
 *      ----- select -----
 *          a. Receive msg from commander, call handle_commander_ui and match
 *          result
 *              1. UiReply::None            => continue
 *
 *              2. UiReply::Reply(id, data) => build_slice_and_write(id, data)
 *
 *              3. UiReply::Flush           => call flush_storage()
 *
 *              4. Err(..)                  => print error
 *
 *          b. Read from stream
 *              1. If Err is Block, continue for other types return
 *              2. While a complete frame is buffered:
 *                  a. Call handle_ui() to get result.
 *                  b. Replace buffer with the unconsumed tail.
 *                  c. match result:
 *                      UiReply::None             => continue
 *
 *                      UiReply::Reply(id, data)  => build_slice_and_write
 *
 *                      UiReply::Flush            => call flush_storage()
 *
 *                      Err(..)                   => If is_ui_error() return,
 *                                                   else continue
 *
 *          c. Check token cancelled,
 *              1. write CLOSE_MSG to stream
 *              2. read one byte to detect peer close, bounded by timeout
 *              3. return Ok
 *
 *  Errors:
 *      UnixSocketError::Read       [b]
 *      UnixSocketError::Write      [a, b]
 */

pub async fn run_module<T>(
    handler: &mut T,
    mut stream: UnixStream,
    token: &CancellationToken,
) -> Result<(), <T as HandleCommander>::Error>
where
    T: FromCommander + HandleCommander + Buffer + HandleUI + FlushStorage,
{
    info!("running");
    handler.buf_as_mut().clear();
    loop {
        select! {
            biased;
            _ = token.cancelled() => {
                let _ = timeout(
                    Duration::from_millis(300),
                    write_and_flush(&mut stream, CLOSE_MSG),
                ).await;
                let mut eof_buf = [0u8; 1];
                let _ = timeout(Duration::from_millis(300), stream.read(&mut eof_buf)).await;
                trace!("runner| cancelled cleanly");
                return Ok(());
            }
            result = handler.recv() => {
                trace!("from commander");
                match result {
                    Some(msg) => {
                        match handler.handle_commander_ui(msg).await
                        {
                            Ok(UiReply::None) => (),
                            Ok(UiReply::Reply(id, data)) => {
                                build_slice_and_write(id, data, &mut stream).await?;
                            }
                            Ok(UiReply::Flush) => {
                                trace!("needs flush");
                                handler.flush_storage(&mut stream).await?;
                            }
                            Err(e) => error!("handle commander| {}", e),
                        }
                    }
                    None => {
                        trace!("commander channel closed");
                        return Ok(());
                    }
                }
            }
            result = stream.readable() => {
                trace!("from socket");
                match read_from_unix(result, &mut stream, handler.buf_as_mut()) {
                    Ok(_) => (),
                    Err(e) => {
                        if matches!(e, UnixSockError::Block) {
                            continue
                        }
                        return Err(e.into());
                    }
                }
                while let Some(tail) = split_json_frame(handler.buf_as_mut()) {
                    let result = handler.handle_ui().await;
                    *handler.buf_as_mut() = tail;
                    trace!("frame consumed");
                    match result {
                        Ok(UiReply::None) => (),
                        Ok(UiReply::Reply(id, data)) => {
                            build_slice_and_write(id, data, &mut stream).await?;
                        }
                        Ok(UiReply::Flush) => {
                            trace!("needs flush");
                            handler.flush_storage(&mut stream).await?;
                        }
                        Err(e) => {
                            if e.is_ui_error() {
                                return Err(e);
                            }
                            error!("handle ui| {}", e);
                        }
                    }
                }
            }
        }
    }
}
