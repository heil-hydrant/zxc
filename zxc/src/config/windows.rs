use serde::{Deserialize, Serialize};

use crate::core::constants::{ADDONS, INTERCEPTOR, REPEATER};

// Enum to represent modules
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Windows {
    Interceptor,
    History,
    Repeater,
    Addons,
}

// Module , command alias , tmux target
pub const WINDOWS: [(Windows, &str, &str); 4] = [
    (Windows::Interceptor, INTERCEPTOR, "0.0"),
    (Windows::History, "vhistory", "1.0"),
    (Windows::Repeater, REPEATER, "2.0"),
    (Windows::Addons, ADDONS, "3.0"),
];
