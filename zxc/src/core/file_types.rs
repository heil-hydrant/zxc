use serde::{Deserialize, Serialize};

pub const EXT_REQ: &str = "req";
pub const EXT_RES: &str = "res";

pub const EXT_WREQ: &str = "wreq";
pub const EXT_WRES: &str = "wres";

// #[derive(Serialize, Deserialize, Debug)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Req,
    Res,
    Wreq,
    Wres,
}
