#![allow(warnings, dead_code)]
#![allow(clippy::result_large_err)]

mod config;
use chrono::Local;
use clap::Parser;
use config::CliArgs;
use config::local::io::write_local_config;
use tmux::Session;
mod addons;
mod async_step;
mod commander;
mod file_types;
mod history;
mod id;
mod interceptor;
mod io;
mod proxy;
mod repeater;
mod setup;
use std::fs::{create_dir, remove_dir_all};
use std::io::Error;
mod builder;
mod forward_info;
mod run;
mod tmux;

use builder::*;
use commander::{CommanderRequest, run_commander};
use run::starter::start_module;
use setup::*;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{Instrument, Level, error, span};
const CAPACITY_2MB: usize = 65536 * 32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = CliArgs::parse()
        .sanitize()
        .unwrap_or_default();

    let attach = args.should_attach();

    let session_name = args
        .session_name(attach)
        .unwrap_or_else(|| {
            format!("zxs-{}", Local::now().format("%d-%m-%H-%M-%S"))
        });

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

    let local_config = args
        .proxy_args
        .take()
        .and_then(|proxy_args| write_local_config(attach, proxy_args));

    let addr = format!("0.0.0.0:{}", port);
    let proxy_listener = TcpListener::bind(addr)
        .await
        .map_err(|e| MainError::PortBind(port, e))?;

    let tmp_path = format!("/tmp/{}", session_name);
    create_dir(&tmp_path).map_err(MainError::TempDir)?;

    let index = if attach {
        get_largest_file_index().map_err(MainError::LargestIndex)? + 1
    } else {
        1
    };

    let mut builder = Builder::new(index, session_name.clone());

    let set = TaskTracker::new();
    let token = CancellationToken::new();
    let _ = setup_logging(args.debug)
        .map_err(|e| eprintln!("setup logging| {}", e));

    // History
    match builder.build_listener(HISTORY) {
        Ok(listener) => {
            let history_handler = builder.build_history();
            let token = token.clone();
            let _abort_history = set.spawn(async move {
                let span = span!(Level::INFO, HISTORY);
                let _ = span.enter();
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
                let _ = span.enter();
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
                let _ = span.enter();
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
                let _ = span.enter();
                let _ = start_module(addon_handler, listener, token)
                    .instrument(span)
                    .await;
            });
        }
        Err(e) => error!("build {}| {}", ADDONS, e),
    }

    let soldier_tx = builder.build_soldier_comm();
    // Commander
    let ctoken = token.clone();
    let commander = builder.build_commander(local_config);
    let _commander_handle = set.spawn(async move {
        let span = span!(Level::INFO, COMMANDER);
        let _ = span.enter();
        run_commander(commander, ctoken)
            .instrument(span)
            .await;
    });

    // Proxy
    let proxy_token = CancellationToken::new();
    let proxy_token_clone = proxy_token.clone();
    let _proxy_handle = set.spawn(async move {
        let span = span!(Level::INFO, PROXY);
        let _ = span.enter();
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

    let handle = tokio::spawn(async move {
        set.close();
        set.wait().await;
    });

    // tmux session
    let session = Session::new(session_name);
    session
        .build(args.debug)
        .map_err(MainError::BuildSession)?;
    session
        .attach_session()
        .map_err(MainError::AttachSession)?;

    proxy_token.cancel();
    handle.await?;
    remove_dir_all(format!("/tmp/{}", session.name))?;
    Ok(())
}

#[inline]
fn create_session_dirs(sname: &str) -> Result<(), Error> {
    create_dir(sname)?;
    let history_path = format!("./{}/history", sname);
    create_dir(&history_path)
}

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error("create session dir| {}", .0)]
    CreateSessionDir(Error),
    #[error("set current dir| {}", .0)]
    CurrentDir(Error),
    #[error("bind to port| {}| {}", .0, .1)]
    PortBind(u16, Error),
    #[error("create temp dir| {}", .0)]
    TempDir(Error),
    #[error("failed to get largest index, May lead to data overwriting| {}", .0)]
    LargestIndex(Error),
    #[error("proxy| {}", .0)]
    Proxy(Error),
    #[error("build session| {}", .0)]
    BuildSession(Error),
    #[error("attach session| {}", .0)]
    AttachSession(Error),
}
