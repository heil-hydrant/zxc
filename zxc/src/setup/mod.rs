use std::fs::read_dir;
use std::io::Error;

mod logging;
pub use logging::*;

pub fn get_largest_file_index() -> Result<usize, Error> {
    Ok(read_dir("history")?
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

#[cfg(test)]
mod tests {
    use std::env::set_current_dir;
    use std::fs::create_dir;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_get_largest_file_index() -> Result<(), Error> {
        let path = PathBuf::from("/tmp/zxc_test");
        set_current_dir(&path)?;
        let result = get_largest_file_index()?;
        assert_eq!(result, 10);

        for i in 11..20 {
            let path = path.join(format!("history/{}", i));
            create_dir(&path)?;
        }
        let result = get_largest_file_index()?;
        assert_eq!(result, 19);
        Ok(())
    }
}
