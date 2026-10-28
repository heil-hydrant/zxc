//#![allow(warnings, dead_code)]
#![allow(async_fn_in_trait)]
#![allow(clippy::result_large_err)]

mod addons;
mod builder;
mod commander;
mod config;
pub mod core;
pub use core::constants::*;
pub use core::{async_step, file_types, forward_info, id};
mod history;
mod interceptor;
mod io;
mod proxy;
mod repeater;
mod run;
mod setup;
mod tmux;

use std::fs::{create_dir, remove_dir_all};
use std::io::Error;
use std::path::PathBuf;
use std::time::Duration;

use builder::*;
use chrono::Local;
use clap::Parser;
use commander::run_commander;
use config::{CliArgs, write_local_config};
use run::starter::start_module;
use setup::*;
use tmux::Session;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{Instrument, Level, debug, error, span};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = CliArgs::parse()
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
    let _dir_guard = SessionDirGuard {
        path: PathBuf::from(&tmp_path),
    };

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

    // tmux session
    let session = Session::new(session_name);
    session
        .build(args.debug)
        .map_err(MainError::BuildSession)?;
    session
        .attach_session()
        .map_err(MainError::AttachSession)?;

    proxy_token.cancel();
    set.close();
    match tokio::time::timeout(Duration::from_secs(3), set.wait()).await {
        Ok(()) => debug!("tasks drained"),
        Err(_) => error!("timed out waiting for tasks to drain"),
    }
    Ok(())
}

struct SessionDirGuard {
    path: PathBuf,
}

impl Drop for SessionDirGuard {
    fn drop(&mut self) {
        if self.path.exists()
            && let Err(e) = remove_dir_all(&self.path)
        {
            eprintln!("remove session dir| {}| {}", self.path.display(), e);
        }
    }
}

fn default_session_name() -> String {
    format!("zxs-{}", Local::now().format("%d-%m-%H-%M-%S"))
}

#[inline(always)]
fn resolve_session_name(
    provided: Option<String>,
    attach: bool,
) -> Result<String, MainError> {
    match provided {
        Some(name) => match validate_session_name(&name) {
            Ok(()) => Ok(name),
            Err(e) if attach => Err(e),
            Err(e) => {
                eprintln!("invalid session name| {}| using default", e);
                Ok(default_session_name())
            }
        },
        None => Ok(default_session_name()),
    }
}

#[inline(always)]
fn validate_session_name(name: &str) -> Result<(), MainError> {
    let valid = !name.is_empty()
        && name.len() <= 64
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(MainError::InvalidSessionName(name.to_string()))
    }
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
    #[error("invalid session name| {0}")]
    InvalidSessionName(String),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_session_name() {
        assert!(validate_session_name("zxs-31-10-12-00-00").is_ok());
        assert!(validate_session_name("abc_123").is_ok());
        assert!(validate_session_name("").is_err());
        assert!(validate_session_name(".").is_err());
        assert!(validate_session_name("..").is_err());
        assert!(validate_session_name("../escape").is_err());
        assert!(validate_session_name("a/b").is_err());

        let long_name = "a".repeat(65);
        assert!(validate_session_name(&long_name).is_err());
    }

    #[test]
    fn test_resolve_session_name() {
        assert_eq!(
            resolve_session_name(Some("valid_name".to_string()), false)
                .unwrap(),
            "valid_name"
        );
        assert_eq!(
            resolve_session_name(Some("valid_name".to_string()), true)
                .unwrap(),
            "valid_name"
        );

        let fallback =
            resolve_session_name(Some("../escape".to_string()), false)
                .unwrap();
        assert!(fallback.starts_with("zxs-"));

        assert!(matches!(
            resolve_session_name(Some("../escape".to_string()), true),
            Err(MainError::InvalidSessionName(_))
        ));

        let default = resolve_session_name(None, false).unwrap();
        assert!(default.starts_with("zxs-"));
    }
}
