use std::collections::HashMap;
use std::fmt::Debug;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use bytes::BytesMut;
use tokio::fs::{copy, create_dir};
use tokio::net::UnixStream;
use tokio::sync::mpsc::Receiver;
use tracing::trace;
use zxc_derive::{Buffer, CloseAction, FlushStorage, NotifyCommander};

use super::error::AddonError;
use super::message::to_ui::AddonMsg;
use crate::config::Addon;
use crate::file_types::EXT_REQ;
use crate::forward_info::{ForwardInfo, Module};
use crate::io::inc_dir::{DirError, incremental};
use crate::io::unix_sock::error::UnixSockError;
use crate::run::boundary::{
    Buffer, CloseAction, FlushStorage, HandleCommander, NotifyCommander
};
use crate::{ADDONS, CAPACITY_2MB};
mod impl_from_commander;
mod impl_handle_commander_msg;
mod impl_handle_ui;

#[derive(Buffer, FlushStorage, CloseAction, NotifyCommander)]
pub struct AddonHandler {
    from_commander: Receiver<ForwardInfo>,
    addons_map: HashMap<String, Addon>,
    buf: BytesMut,
}

impl AddonHandler {
    #[inline(always)]
    pub fn new(
        from_commander: Receiver<ForwardInfo>,
        addons_map: HashMap<String, Addon>,
    ) -> AddonHandler {
        AddonHandler {
            from_commander,
            addons_map,
            buf: BytesMut::with_capacity(CAPACITY_2MB),
        }
    }

    /* Steps:
     *      1. Check if the module is an addon
     *      2. Get the addon from the map
     *      3. Build incremental destination file, by calling build_addon_dest
     *      4. Copy info.file to dest_file
     *      5. Get tls from server_info, if None, set it to true
     *      6. Build addon command string by calling build_addon_cmd
     *      with addon_name, dest_file and is_tls
     *      7. Build new AddonMsg and serialize to string and return
     *
     * Error:
     *      AddonError::WrongMsg        [1]
     *      AddonError::AddonNotFound   [2]
     *      AddonError::DirError        [3] [4]
     *      AddonError::Serialize       [7]
     */

    pub async fn build_cmd_string(
        &self,
        info: ForwardInfo,
    ) -> Result<String, AddonError> {
        let name = match info.to_module() {
            Module::Addon(name) => name,
            _ => return Err(AddonError::WrongMsg(info)),
        };

        let addon = self
            .addons_map
            .get(name)
            .ok_or_else(|| AddonError::AddonNotFound(name.to_string()))?;

        let dest_file = build_addon_dest(&addon.prefix, &info.file).await?;

        copy(&info.file, &dest_file)
            .await
            .map_err(DirError::CopyFile)?;

        let is_tls = info
            .server_info
            .as_ref()
            .map(|info| info.is_tls())
            .unwrap_or(true);

        let cmd = addon.build_cmd(name, &dest_file.to_string_lossy(), is_tls);

        let addon_msg = AddonMsg::new(cmd, dest_file);
        Ok(serde_json::to_string(&addon_msg)?)
    }
}

/* Steps:
 *      1. Check if the path has enough components, i.e. 3
 *              ./history/1/1.req
 *
 *      2. Pop (component_count - 3) times
 *              ./history/1
 *
 *      3. Push addons to the path and create_dir, if Error is AlreadyExists
 *      do nothing.
 *              ./history/1/addons
 *
 *      4. Get Incremental file for the addon prefix
 *
 *      5. Push prefix and incremental to the path
 *              ./history/1/addons/$prefix-$incremental.req
 *
 * Error:
 *      DirError::NoTop             [1]
 *      DirError::CreateDir         [3]
 *      DirError::GetIncrementalDir [4]
 */

pub async fn build_addon_dest(
    prefix: &str,
    src_file: &Path,
) -> Result<PathBuf, DirError> {
    let mut path = src_file.to_path_buf();

    if path.components().count() < 3 {
        return Err(DirError::NoTop(src_file.to_string_lossy().to_string()));
    }
    for _ in 0..(path.components().count() - 3) {
        path.pop();
    }
    trace!("top| {}", &path.to_string_lossy());
    path.push(ADDONS);
    if let Err(e) = create_dir(&path).await
        && !matches!(e.kind(), ErrorKind::AlreadyExists)
    {
        return Err(DirError::CreateDir(e));
    }
    let incremental = incremental(&path, prefix, false)
        .await
        .map_err(DirError::IncrementalDir)?;
    path.push(format!("{}{}.{}", prefix, incremental, EXT_REQ));
    Ok(path)
}

impl Debug for AddonHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Addon")
    }
}

#[cfg(test)]
pub mod tests {
    use std::env::set_current_dir;
    use std::error::Error;
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn test_zz_incremental_file_last() -> Result<(), Box<dyn Error>> {
        let test_dir = PathBuf::from("/tmp/zxc_test");
        set_current_dir(&test_dir)?;

        let prefixes = [("q-", "sqlmap"), ("z-", "ffuf")];
        let mut path = PathBuf::from("./history/");

        for i in 1..11 {
            path.push(i.to_string());
            path.push(i.to_string());
            path.set_extension("req");

            for (prefix, name) in &prefixes {
                let result = build_addon_dest(prefix, &path).await?;
                let verify = PathBuf::from(format!(
                    "./history/{}/addons/{}{}.req",
                    i,
                    prefix,
                    i + 1
                ));
                assert_eq!(
                    result, verify,
                    "{} test failed for index {}",
                    name, i
                );
            }

            path.pop();
            path.pop();
        }
        Ok(())
    }
}
