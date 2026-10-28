use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::process::{Command, ExitStatus};
use std::thread::sleep;
use std::time::{Duration, Instant};
mod cmd;
mod constants;
use cmd::{
    attach_session, create_session, create_window, list_sessions,
    list_windows, rename_window, run_module, set_env,
};
use constants::*;

use crate::config::WINDOWS;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub struct Session {
    pub name: String,
}

impl Session {
    pub fn new(name: String) -> Session {
        Session {
            name,
        }
    }

    pub fn build(&self, debug: bool) -> Result<(), Error> {
        // session
        self.create_session()?;
        self.sleep_till_session_creation()?;
        self.set_env("ZXC", "1")?;
        if debug {
            self.set_env("ZXC_DEBUG", "1")?;
        }
        // windows
        self.create_windows()?;
        self.sleep_till_windows_creation()?;
        self.start_modules()
    }

    // Gets tmux command for the session:  tmux -L session_name
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::new(TMUX);
        cmd.arg(SOCKET_FLAG).arg(&self.name);
        cmd
    }

    // Cmd: tmux -L session_name -f $HOME/config/tmux.conf new-session -d
    #[inline]
    pub fn create_session(&self) -> Result<(), Error> {
        let mut cmd = self.cmd();
        create_session(&mut cmd)?;
        ensure_success(cmd.status()?, "create session")
    }

    pub fn sleep_till_session_creation(&self) -> Result<(), Error> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let mut cmd = self.cmd();
        list_sessions(&mut cmd);
        loop {
            let status = cmd.status()?;
            if status.success() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Error::new(
                    ErrorKind::TimedOut,
                    "tmux session creation timed out",
                ));
            }
            sleep(POLL_INTERVAL);
        }
    }

    pub fn create_windows(&self) -> Result<(), Error> {
        for (index, (_, name, _)) in WINDOWS.iter().enumerate() {
            if index != 0 {
                let mut cmd = self.cmd();
                // tmux -L session_name new-window
                create_window(&mut cmd);
                ensure_success(cmd.status()?, "create window")?;
            }
            let mut cmd = self.cmd();
            // tmux -L session_name rename-window -t $windows_id $module
            rename_window(&mut cmd, index, name);
            ensure_success(cmd.status()?, "rename window")?;
        }
        Ok(())
    }

    pub fn sleep_till_windows_creation(&self) -> Result<(), Error> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let mut cmd = self.cmd();
        // tmux -L session_name list-windows -F "#{window_index}"
        list_windows(&mut cmd);

        loop {
            let result = cmd.output()?;
            ensure_success(result.status, "list windows")?;
            if BufReader::new(result.stdout.as_slice())
                .lines()
                .count()
                == 4
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Error::new(
                    ErrorKind::TimedOut,
                    "tmux window creation timed out",
                ));
            }
            sleep(POLL_INTERVAL);
        }
    }

    pub fn start_modules(&self) -> Result<(), Error> {
        for (_, name, target) in WINDOWS.iter() {
            let mut cmd = self.cmd();
            // tmux -L session_name send-keys -t module.target() module Enter
            run_module(&mut cmd, target, name);
            ensure_success(cmd.status()?, "start module")?;
        }
        Ok(())
    }

    pub fn set_env(&self, name: &str, value: &str) -> Result<(), Error> {
        let mut cmd = self.cmd();
        // tmux -L session_name setenv env_var env_value
        set_env(&mut cmd, name, value);
        ensure_success(cmd.status()?, "set env")
    }

    // tmux -L session_name attach -t :1
    // By default, tmux attaches to the history window (:1).
    pub fn attach_session(&self) -> Result<ExitStatus, Error> {
        let mut cmd = self.cmd();
        attach_session(&mut cmd);
        cmd.status()
    }
}

fn ensure_success(status: ExitStatus, action: &str) -> Result<(), Error> {
    if status.success() {
        Ok(())
    } else {
        Err(Error::other(format!("tmux {action} failed| {status}")))
    }
}

#[cfg(test)]
mod holas {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn hola_cmd() {
        let session = Session::new("hola".to_string());
        let binding = session.cmd();
        let args: Vec<&OsStr> = binding.get_args().collect();
        let verify = vec!["-L", "hola"];
        assert_eq!(verify, args);
    }
}
