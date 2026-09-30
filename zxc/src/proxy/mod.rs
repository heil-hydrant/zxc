pub mod handler_state;
pub mod server_info;
pub mod states;

use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, Level, debug, error, span, trace};

use crate::CommanderRequest;
use crate::async_step::async_run;
use crate::proxy::states::ConnectionState;

pub async fn start_proxy(
    tx: Sender<CommanderRequest>,
    listener: TcpListener,
    token: CancellationToken,
    task_token: CancellationToken,
) -> std::io::Result<()> {
    debug!("[*] Proxy Started");
    let mut tasks = tokio::task::JoinSet::new();
    let mut id = 1;
    loop {
        select! {
            _ = token.cancelled() => {
                debug!("[*] Proxy Stopped");
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let tx_clone = tx.clone();
                        tasks.spawn(async move {
                            let span = span!(Level::TRACE, "Prx", id);
                            let state = ConnectionState::<TcpStream>::new(
                                id,
                                stream,
                                tx_clone.clone(),
                            );
                            if let Err(e) = async_run(state).instrument(span).await {
                                if e.is_common_error() {
                                    trace!("{}", e)
                                } else {
                                    error!("{}", e)
                                }
                            }
                            let request = CommanderRequest::Close(id);
                            let _ = tx_clone.send(request).await;
                        });
                        id += 1;
                    }
                    Err(e) => {
                        error!("accept| {}", e);
                    }
                }
            }
            res = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(Err(e)) = res
                    && !e.is_cancelled() {
                        error!("task panicked| {e}");
                    }
            }
        }
    }
    tasks.abort_all();
    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res
            && !e.is_cancelled()
        {
            error!("task shutdown| {e}");
        }
    }
    task_token.cancel();
    Ok(())
}
