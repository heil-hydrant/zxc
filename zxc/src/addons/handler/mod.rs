mod impl_from_commander;
mod impl_handle_commander_msg;
mod impl_handle_ui;

use std::collections::HashMap;
use std::fmt::Debug;

use bytes::BytesMut;
use tokio::fs::copy;
use tokio::net::UnixStream;
use tokio::sync::mpsc::Receiver;
use zxc_derive::{Buffer, CloseAction, FlushStorage, NotifyCommander};

use super::AddonError;
use super::builder::build_addon_dest;
use super::message::to_ui::AddonMsg;
use crate::CAPACITY_2MB;
use crate::config::Addon;
use crate::forward_info::{ForwardInfo, Module};
use crate::io::{DirError, UnixSockError};
use crate::run::boundary::{
    Buffer, CloseAction, FlushStorage, HandleCommander, NotifyCommander
};

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

impl Debug for AddonHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Addon")
    }
}
