use tokio::io::AsyncReadExt;
use tracing::{error, info, trace};

use super::boundary::*;
use crate::io::*;

const CLOSE_MSG: &[u8] = br#"[0,{"Action": "Close"}]"#;

pub async fn run_module<T>(
    handler: &mut T,
    mut stream: tokio::net::UnixStream,
    token: &tokio_util::sync::CancellationToken,
) -> Result<(), <T as HandleCommander>::Error>
where
    T: FromCommander + HandleCommander + Buffer + HandleUI + FlushStorage,
{
    info!("running");
    handler.buf_as_mut().clear();
    loop {
        tokio::select! {
            biased;
            _ = token.cancelled() => {
                let _ = tokio::time::timeout(
                    std::time::Duration::from_millis(300),
                    write_and_flush(&mut stream, CLOSE_MSG),
                ).await;
                let mut eof_buf = [0u8; 1];
                let _ = tokio::time::timeout(std::time::Duration::from_millis(300), stream.read(&mut eof_buf)).await;
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
                while let Some(buf) = split_json_frame(handler.buf_as_mut()) {
                    let result = handler.handle_ui().await;
                    *handler.buf_as_mut() = buf;
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
