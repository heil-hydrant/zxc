pub mod io;
pub mod proxy;
mod session;

use clap::Parser;

use self::session::SessionArgs;
use crate::config::*;

// Struct for cli args
#[derive(Parser, Debug, Default)]
pub struct CliArgs {
    #[command(flatten)]
    pub session_args: Option<SessionArgs>,
    #[command(flatten)]
    pub proxy_args: Option<ProxyArgs>,
    /// Debug mode
    #[arg(short, long = "debug")]
    pub debug: bool,
}

impl CliArgs {
    /* Since, session_args.new_name and session_args.attach_name are mutually
     * exclusive and are checked for empty values using serde, no need to
     * sanitize them.
     */

    pub fn sanitize(mut self) -> Option<Self> {
        self.proxy_args = self
            .proxy_args
            .take()
            .and_then(|proxy_args| proxy_args.sanitize());

        if self.debug
            || self.session_args.is_some()
            || self.proxy_args.is_some()
        {
            Some(self)
        } else {
            None
        }
    }

    pub fn should_attach(&self) -> bool {
        self.session_args
            .as_ref()
            .is_some_and(|s| s.attach_name.is_some())
    }

    pub fn session_name(&mut self, should_attach: bool) -> Option<String> {
        self.session_args.as_mut().map(|sess| {
            if should_attach {
                sess.attach_name.take().unwrap()
            } else {
                sess.new_name.take().unwrap()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize() {
        let args = CliArgs::default();
        assert!(args.sanitize().is_none());
    }

    #[test]
    fn test_sanitize_debug() {
        let args = CliArgs {
            debug: true,
            ..Default::default()
        };
        assert!(args.sanitize().is_some());
    }

    #[test]
    fn test_sanitize_proxy() {
        let prxy = ProxyArgs::default();
        let args = CliArgs {
            proxy_args: Some(prxy),
            ..Default::default()
        };
        assert!(args.sanitize().is_none());
    }

    #[test]
    fn test_should_attach_no_args() {
        let args = CliArgs::default();
        assert!(!args.should_attach());
    }

    #[test]
    fn test_should_attach_attach() {
        let sess = SessionArgs {
            attach_name: Some("test".to_string()),
            ..Default::default()
        };
        let args = CliArgs {
            session_args: Some(sess),
            ..Default::default()
        };
        assert!(args.should_attach());
    }

    #[test]
    fn test_cli_args_session_name_no_name() {
        let mut args = CliArgs::default();
        let name = args.session_name(false);
        assert!(name.is_none());
    }

    #[test]
    fn test_cli_args_session_name_attach() {
        let session_args = SessionArgs {
            attach_name: Some("test".to_string()),
            ..Default::default()
        };

        let mut args = CliArgs {
            session_args: Some(session_args),
            ..Default::default()
        };
        let name = args.session_name(true).unwrap();
        assert_eq!(name, "test");
    }

    #[test]
    fn test_cli_args_session_name_new() {
        let session_args = SessionArgs {
            new_name: Some("test".to_string()),
            ..Default::default()
        };
        let mut args = CliArgs {
            session_args: Some(session_args),
            ..Default::default()
        };

        let name = args.session_name(false).unwrap();
        assert_eq!(name, "test");
    }
}
