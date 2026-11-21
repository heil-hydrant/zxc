mod impl_boundary;

use std::collections::HashMap;
use std::fmt::Debug;

use bytes::BytesMut;
use tokio::net::UnixStream;
use tokio::sync::mpsc::Receiver;
use zxc_derive::{Buffer, CloseAction, FlushStorage, NotifyCommander};

use super::builder::build_addon_dest;
use super::h2::ensure_host_file;
use super::message::to_ui::AddonMsg;
use super::AddonError;
use crate::config::Addon;
use crate::forward_info::{ForwardInfo, Module};
use crate::io::{copy_with_request_metadata, DirError, UnixSockError};
use crate::run::boundary::{
    Buffer, CloseAction, FlushStorage, HandleCommander, NotifyCommander,
};
use crate::CAPACITY_2MB;

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

    /// Builds the serialized addon command for a forwarded request.
    ///
    /// # Steps
    ///
    /// 1. Check if the module is an addon.
    /// 2. Get the addon from the map.
    /// 3. Build the incremental destination file with
    ///    [`build_addon_dest`].
    /// 4. Copy `info.file` to `dest_file`, preserving request metadata.
    /// 5. Read the HTTP/2 flag from `is_h2`, which is `true` only when the
    ///    UI forwarded an HTTP/2 request.
    /// 6. For HTTP/2, ensure `dest_file` has a `Host` header.
    /// 7. Get the TLS flag from `server_info`, defaulting to `true` when
    ///    absent.
    /// 8. Build the addon command string with `Addon::build_cmd` from the
    ///    addon name, `dest_file`, the TLS flag and `is_h2`.
    /// 9. Build a new `AddonMsg` and return it serialized.
    ///
    /// # Errors
    ///
    /// - `AddonError::WrongMsg` [1]
    /// - `AddonError::AddonNotFound` [2]
    /// - `AddonError::DirError` [3] [4]
    /// - `AddonError::H2Request` [6]
    /// - `AddonError::RequestFile` [6]
    /// - `AddonError::Serialize` [9]
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

        let addon_msg = AddonMsg::new(cmd, dest_file);
        Ok(serde_json::to_string(&addon_msg)?)
    }
}

impl Debug for AddonHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Addon")
    }
}
