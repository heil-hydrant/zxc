use std::{io::Error, path::PathBuf};

use crate::MainError;

pub struct SessionDirGuard {
    pub path: PathBuf,
}

impl Drop for SessionDirGuard {
    fn drop(&mut self) {
        if self.path.exists()
            && let Err(e) = std::fs::remove_dir_all(&self.path)
        {
            eprintln!("remove session dir| {}| {}", self.path.display(), e);
        }
    }
}

fn default_session_name() -> String {
    format!("zxs-{}", chrono::Local::now().format("%d-%m-%H-%M-%S"))
}

#[inline(always)]
pub(super) fn resolve_session_name(
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
pub(super) fn validate_session_name(name: &str) -> Result<(), MainError> {
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

pub fn get_largest_file_index() -> Result<usize, Error> {
    Ok(std::fs::read_dir("history")?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_dir() {
                path.file_name()?
                    .to_str()?
                    .parse::<usize>()
                    .ok()
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0))
}

#[inline]
pub fn create_session_dirs(sname: &str) -> Result<(), Error> {
    use std::fs::create_dir;
    create_dir(sname)?;
    let history_path = format!("./{}/history", sname);
    create_dir(&history_path)
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

    #[test]
    fn test_get_largest_file_index() -> Result<(), Error> {
        let path = PathBuf::from("/tmp/zxc_test");
        std::env::set_current_dir(&path)?;
        let result = get_largest_file_index()?;
        assert_eq!(result, 10);

        for i in 11..20 {
            let path = path.join(format!("history/{}", i));
            std::fs::create_dir(&path)?;
        }
        let result = get_largest_file_index()?;
        assert_eq!(result, 19);
        Ok(())
    }
}
