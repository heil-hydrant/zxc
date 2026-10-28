use std::fmt::Debug;

use tokio::net::UnixListener;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

use super::boundary::*;
use crate::run::runner::run_module;

pub async fn start_module<T>(
    mut handler: T,
    listener: UnixListener,
    token: CancellationToken,
) where
    T: FromCommander
        + HandleCommander
        + Buffer
        + HandleUI
        + FlushStorage
        + CloseAction
        + NotifyCommander
        + Debug,
{
    debug!("started");
    loop {
        select! {
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
                        match run_module(&mut handler, &listener, conn, &token).await {
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
