#![allow(async_fn_in_trait)]
#![allow(clippy::result_large_err)]

mod addons;
mod builder;
mod commander;
mod config;
pub mod core;
mod history;
mod interceptor;
mod io;
mod logging;
mod proxy;
mod repeater;
mod run;
mod session;
mod tmux;

pub use core::constants::*;
pub use core::{async_step, file_types, forward_info, id};
pub use session::*;

use std::io::Error;
use std::path::PathBuf;

use builder::*;
use clap::Parser;
use run::starter::start_module;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, Level, debug, error, span};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), MainError> {
    let mut args = config::CliArgs::parse()
        .sanitize()
        .unwrap_or_default();

    let attach = args.should_attach();

    let session_name =
        resolve_session_name(args.session_name(attach), attach)?;

    if !attach {
        create_session_dirs(&session_name)
            .map_err(MainError::CreateSessionDir)?;
    }

    std::env::set_current_dir(&session_name).map_err(MainError::CurrentDir)?;

    let port: u16 = args
        .proxy_args
        .as_mut()
        .and_then(|proxy_arg| proxy_arg.port.take())
        .unwrap_or(8080);

    let listen = args
        .proxy_args
        .as_mut()
        .and_then(|proxy_arg| proxy_arg.listen.take())
        .unwrap_or(DEFAULT_LISTEN);

    let local_config = args
        .proxy_args
        .take()
        .and_then(|proxy_args| config::write_local_config(attach, proxy_args));

    let addr = std::net::SocketAddr::new(listen, port);
    let proxy_listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| MainError::PortBind(port, e))?;

    let tmp_path = format!("/tmp/{}", session_name);
    std::fs::create_dir(&tmp_path).map_err(MainError::TempDir)?;
    let _dir_guard = SessionDirGuard {
        path: PathBuf::from(&tmp_path),
    };

    let index = if attach {
        get_largest_file_index().map_err(MainError::LargestIndex)? + 1
    } else {
        1
    };

    let mut builder = Builder::new(index, session_name.clone())
        .map_err(MainError::Crypto)?;

    let set = tokio_util::task::TaskTracker::new();
    let token = CancellationToken::new();
    let _ = logging::setup_logging(args.debug)
        .map_err(|e| eprintln!("setup logging| {}", e));

    // History
    match builder.build_listener(HISTORY) {
        Ok(listener) => {
            let history_handler = builder.build_history();
            let token = token.clone();
            let _abort_history = set.spawn(async move {
                let span = span!(Level::INFO, HISTORY);
                let _ = start_module(history_handler, listener, token)
                    .instrument(span)
                    .await;
            });
        }
        Err(e) => error!("build {}| {}", HISTORY, e),
    }

    // Interceptor
    match builder.build_listener(INTERCEPTOR) {
        Ok(listener) => {
            let interceptor_handler = builder.build_interceptor();
            let token = token.clone();
            let _abort_interceptor = set.spawn(async move {
                let span = span!(Level::INFO, INTERCEPTOR);
                let _ = start_module(interceptor_handler, listener, token)
                    .instrument(span)
                    .await;
            });
        }
        Err(e) => error!("build {}| {}", INTERCEPTOR, e),
    }

    // Repeater
    match builder.build_listener(REPEATER) {
        Ok(listener) => {
            let repeater_handler = builder.build_repeater();
            let token = token.clone();
            let _repeater_handle = set.spawn(async move {
                let span = span!(Level::INFO, REPEATER);
                let _ = start_module(repeater_handler, listener, token)
                    .instrument(span)
                    .await;
            });
        }
        Err(e) => error!("build {}| {}", REPEATER, e),
    }

    // Addons
    builder.parse_global_config();
    match builder.build_listener(ADDONS) {
        Ok(listener) => {
            let addon_handler = builder.build_addons();
            let token = token.clone();
            let _addon_handle = set.spawn(async move {
                let span = span!(Level::INFO, ADDONS);
                let _ = start_module(addon_handler, listener, token)
                    .instrument(span)
                    .await;
            });
        }
        Err(e) => error!("build {}| {}", ADDONS, e),
    }

    let (soldier_tx, soldier_rx) = builder.build_soldier_comm();
    // Commander
    let ctoken = token.clone();
    let commander = builder
        .build_commander(local_config, soldier_rx)
        .map_err(MainError::Builder)?;
    let _commander_handle = set.spawn(async move {
        let span = span!(Level::INFO, COMMANDER);
        commander::run_commander(commander, ctoken)
            .instrument(span)
            .await;
    });

    // Proxy
    let proxy_token = CancellationToken::new();
    let proxy_token_clone = proxy_token.clone();
    let _proxy_handle = set.spawn(async move {
        let span = span!(Level::INFO, PROXY);
        proxy::start_proxy(
            soldier_tx,
            proxy_listener,
            proxy_token_clone,
            token,
        )
        .instrument(span)
        .await
        .map_err(MainError::Proxy)
        .unwrap();
    });

    // tmux session
    let session = tmux::Session::new(session_name);
    session
        .build(args.debug)
        .map_err(MainError::BuildSession)?;
    session
        .attach_session()
        .map_err(MainError::AttachSession)?;

    proxy_token.cancel();
    set.close();
    match tokio::time::timeout(std::time::Duration::from_secs(3), set.wait())
        .await
    {
        Ok(()) => debug!("tasks drained"),
        Err(_) => error!("timed out waiting for tasks to drain"),
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error("session directory: {}", .0)]
    CreateSessionDir(Error),
    #[error("CA initialization: {0}")]
    Crypto(commander::captain_crypto::CryptoBuildError),
    #[error("builder: {0}")]
    Builder(BuilderError),
    #[error("invalid session name: {0}")]
    InvalidSessionName(String),
    #[error("current directory: {}", .0)]
    CurrentDir(Error),
    #[error("bind to port {0}: {1}")]
    PortBind(u16, Error),
    #[error("temporary directory: {}", .0)]
    TempDir(Error),
    #[error("largest index (data may be overwritten): {}", .0)]
    LargestIndex(Error),
    #[error("proxy: {}", .0)]
    Proxy(Error),
    #[error("session build: {}", .0)]
    BuildSession(Error),
    #[error("session attach: {}", .0)]
    AttachSession(Error),
}

#[cfg(test)]
mod tests {
    use crate::config::CliArgs;
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    #[test]
    fn test_cli_listen_parses_ip() {
        let args = CliArgs::parse_from(["zxc", "--listen", "0.0.0.0"]);
        let listen = args.proxy_args.and_then(|p| p.listen);
        assert_eq!(listen, Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
    }

    #[test]
    fn test_cli_listen_absent() {
        let args = CliArgs::parse_from(["zxc"]);
        let listen = args.proxy_args.and_then(|p| p.listen);
        assert_eq!(listen, None);
        assert_eq!(
            listen.unwrap_or(DEFAULT_LISTEN),
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
    }

    #[test]
    fn test_cli_listen_rejects_host_and_port() {
        let result =
            CliArgs::try_parse_from(["zxc", "--listen", "0.0.0.0:8080"]);
        assert!(result.is_err());
    }
}
