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

/// Runs the event loop for one UI module over its Unix socket.
///
/// # Steps
///
/// 1. Select on commander messages, socket readability, and cancellation.
/// 2. Commander message: call `handle_commander_ui` and handle the reply.
///    `None` continues, `Reply` writes a framed reply, `Flush` drains queued
///    storage, and errors are logged.
/// 3. Socket read: decode every complete buffered frame, call `handle_ui`,
///    keep the unconsumed tail, and write or flush the reply. Return on a UI
///    error, otherwise continue.
/// 4. Cancellation: write `CLOSE_MSG`, wait briefly for the peer to close,
///    and return.
///
/// # Errors
///
/// - `UnixSocketError::Read` from the socket read.
/// - `UnixSocketError::Write` from commander replies and the close message.
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
                                build_slice_and_write(id, &data, &mut stream).await?;
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
                            build_slice_and_write(id, &data, &mut stream).await?;
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
