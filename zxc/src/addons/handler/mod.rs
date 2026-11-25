mod impl_boundary;

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use bytes::BytesMut;
use tracing::trace;

use tokio::net::UnixStream;
use tokio::sync::mpsc::Receiver;
use zxc_derive::{Buffer, CloseAction, FlushStorage, NotifyCommander};

use super::AddonError;
use super::h2::ensure_host_file;
use super::messages::AddonToUi;
use crate::ADDONS;
use crate::CAPACITY_2MB;
use crate::config::Addon;
use crate::file_types::EXT_REQ;
use crate::forward_info::{ForwardInfo, Module};
use crate::io::*;
use crate::io::{DirError, incremental};
use crate::run::boundary::*;

#[derive(Buffer, FlushStorage, CloseAction, NotifyCommander)]
pub struct AddonHandler {
    addons_map: HashMap<String, Addon>,
    buf: BytesMut,
    from_commander: Receiver<ForwardInfo>,
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

    /// Builds the serialized addon command for a forwarded request.
    pub(super) async fn build_cmd_string(
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

        copy_with_request_metadata(&info.file, &dest_file)
            .await
            .map_err(DirError::CopyFile)?;

        let is_h2 = info.is_h2 == Some(true);
        if is_h2 {
            ensure_host_file(&dest_file, &info).await?;
        }

        let is_tls = info
            .server_info
            .as_ref()
            .map(|info| info.is_tls())
            .unwrap_or(true);

        let cmd =
            addon.build_cmd(name, &dest_file.to_string_lossy(), is_tls, is_h2);

        let addon_msg = AddonToUi::new(cmd, dest_file);
        Ok(serde_json::to_string(&addon_msg)?)
    }
}

impl Debug for AddonHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Addon")
    }
}

/// Builds the destination path for a copied addon request.
async fn build_addon_dest(
    prefix: &str,
    src_file: &Path,
) -> Result<PathBuf, DirError> {
    let mut path = src_file.to_path_buf();

    if path.components().count() < 3 {
        return Err(DirError::NoTop(src_file.to_string_lossy().to_string()));
    }

    // Pop `component_count - 3` components to reach `./history/1`.
    for _ in 0..(path.components().count() - 3) {
        path.pop();
    }
    trace!("top| {}", &path.to_string_lossy());
    path.push(ADDONS);
    if let Err(e) = tokio::fs::create_dir(&path).await
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
