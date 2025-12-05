use std::fmt::Debug;

use tokio::net::UnixListener;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

use super::boundary::*;
use crate::run::runner::run_module;

/* Steps:
 * ----- loop -----
 *      ----- select -----
 *          1. Accept new connections
 *              a. Flush storage
 *              b. call run_module
 *          2. Receive from commander, call handle_commander_no_ui
 *          3. Check token cancelled
 *              return
 */

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
            result = listener.accept() => {
                match result {
                    Ok((mut conn,_)) => {
                        info!("sock connected");
                        if let Err(e) = handler.flush_storage(&mut conn).await{
                            error!("flush storage| {}", e);
                        }
                        if let Err(e) = run_module(&mut handler, &listener, conn, &token).await {
                            if e.is_ui_error()
                                && let Err(e) = handler.notify_commander().await {
                                    error!("notify commander| {}", e);
                                }
                            error!("run| {}", e);
                        }
                    }
                    Err(e) => {
                        error!("accept| {}", e);
                    }
                }
            }
            result = handler.recv()=> {
                if let Some(msg) = result
                    && let Err(e) = handler.handle_commander_no_ui(msg).await{
                        error!("handle commander no ui| {}", e);
                    }
            }
            _ = token.cancelled() => {
                handler.close_action().await.expect("close action");
                trace!("start cancelled");
                return
            }
        }
    }
}
