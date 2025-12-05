use std::fs::read_dir;
use std::io::Error;
mod logging;
pub use logging::*;

/* Steps:
 *      1. Iterate through history directory
 *      2. Check if entry is a directory
 *      3. Get FileName
 *      4. convert to str
 *      5. parse to usize
 *      6. get largest
 */

pub fn get_largest_file_index() -> Result<usize, Error> {
    read_dir("history")?
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
        .ok_or_else(|| Error::other("Failed to get largest index"))
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
