use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ============================================================================
// Messages from Addons UI
// ============================================================================

// Only Close operation supported
#[derive(Deserialize, Debug)]
pub struct AddonUiMsg {
    _id: u8,
    _operation: String,
}

// ============================================================================
// Messages to Addons UI
// ============================================================================

#[derive(Serialize, Debug)]
pub struct AddonToUi {
    cmd: String,
    file: PathBuf,
}

impl AddonToUi {
    pub fn new(cmd: String, file: PathBuf) -> Self {
        Self {
            cmd,
            file,
        }
    }
}
