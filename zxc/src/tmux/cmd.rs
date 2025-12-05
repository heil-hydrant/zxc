use std::env;
use std::fs::metadata;
use std::io::Error;
use std::process::Command;

use super::constants::*;

/* Command:
 *      tmux -L session_name -f $HOME/.config/zxc/tmux.conf new-session -d
 */

#[inline(always)]
pub fn create_session(cmd: &mut Command) -> Result<(), Error> {
    // get users home dir
    let home = env::var("HOME")
        .map_err(|e| Error::other(format!("Get Home Dir| {}", e)))?;
    let config = format!("{}/.config/zxc/tmux.conf", home);
    // check if config file exists
    metadata(&config)?;
    cmd.arg("-f")
        .arg(config)
        .arg(NEW_SESSION)
        .arg(FLAG_DETACH);
    Ok(())
}

// Command | tmux -L session_name ls

#[inline(always)]
pub fn list_sessions(cmd: &mut Command) {
    cmd.arg("ls");
}

// Command | tmux -L session_name list-windows -F "#{window_index}"

#[inline(always)]
pub fn list_windows(cmd: &mut Command) {
    cmd.arg("list-windows")
        .arg("-F")
        .arg("#{window_index}");
}

// Command | tmux -L session_name new-window

pub fn create_window(cmd: &mut Command) {
    cmd.arg(NEW_WINDOW);
}

// Command | tmux -L session_name rename-window -t target_window module

pub fn rename_window(cmd: &mut Command, target: usize, name: &str) {
    cmd.arg(RENAME_WINDOW)
        .arg(FLAG_TARGET)
        .arg(target.to_string())
        .arg(name);
}

// Command | tmux -L session_name send-keys -t target_window module Enter

pub fn run_module(cmd: &mut Command, target: &str, name: &str) {
    cmd.args(ARGUMENTS_BEFORE_TARGET)
        .arg(target)
        .arg(name)
        .arg(ENTER);
}

// Command | tmux -L session_name setenv ZXC 1

pub fn set_env(cmd: &mut Command, name: &str, value: &str) {
    cmd.arg("setenv").arg(name).arg(value);
}

/* Command | tmux -L session_name attach -t :1
 *
 * By default, attaches to the history window (:1).
 */

#[inline(always)]
pub fn attach_session(cmd: &mut Command) {
    cmd.arg("attach").arg("-t").arg(":1");
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn test_create_window() {
        let mut cmd = Command::new(TMUX);
        create_window(&mut cmd);
        let verify = vec!["new-window"];
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert_eq!(verify, args);
    }

    #[test]
    fn test_rename_window() {
        let mut cmd = Command::new(TMUX);
        rename_window(&mut cmd, 1, "repeater");
        let verify = vec!["rename-window", "-t", "1", "repeater"];
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert_eq!(verify, args);
    }

    #[test]
    fn test_run_module() {
        let mut cmd = Command::new(TMUX);
        run_module(&mut cmd, "1", "repeater");
        let verify = vec!["send-keys", "-t", "1", "repeater", "Enter"];
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert_eq!(verify, args);
    }

    #[test]
    fn test_set_env() {
        let mut cmd = Command::new(TMUX);
        set_env(&mut cmd, "ZXC", "1");
        let verify = vec!["setenv", "ZXC", "1"];
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert_eq!(verify, args);
    }

    #[test]
    fn test_attach_session() {
        let mut cmd = Command::new(TMUX);
        attach_session(&mut cmd);
        let verify = vec!["attach", "-t", ":1"];
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert_eq!(verify, args);
    }
}
