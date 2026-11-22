use tracing::{error, info, trace};

use super::boundary::*;

pub async fn start_module<T>(
    mut handler: T,
    listener: tokio::net::UnixListener,
    token: tokio_util::sync::CancellationToken,
) where
    T: FromCommander
        + HandleCommander
        + Buffer
        + HandleUI
        + FlushStorage
        + CloseAction
        + NotifyCommander
        + std::fmt::Debug,
{
    loop {
        tokio::select! {
            biased;
            _ = token.cancelled() => {
                run_close_action(&mut handler).await;
                trace!("start cancelled");
                return;
            }
            result = listener.accept() => {
                match result {
                    Ok((mut conn,_)) => {
                        info!("sock connected");
                        if let Err(e) = handler.flush_storage(&mut conn).await{
                            error!("flush storage| {}", e);
                        }
                        match crate::run::runner::run_module(&mut handler, conn, &token).await {
                            Ok(()) => {
                                run_close_action(&mut handler).await;
                                trace!("start cancelled");
                                return;
                            }
                            Err(e) => {
                                if e.is_ui_error()
                                    && let Err(e) = handler.notify_commander().await {
                                        error!("notify commander| {}", e);
                                }
                                error!("run| {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("accept| {}", e);
                    }
                }
            }
            result = handler.recv() => {
                match result {
                    Some(msg) => {
                        if let Err(e) = handler.handle_commander_no_ui(msg).await {
                            error!("handle commander no ui| {}", e);
                        }
                    }
                    None => {
                        run_close_action(&mut handler).await;
                        trace!("commander channel closed");
                        return;
                    }
                }
            }
        }
    }
}

async fn run_close_action<T>(handler: &mut T)
where
    T: CloseAction,
{
    if let Err(e) = handler.close_action().await {
        error!("close action| {}", e);
    }
}
