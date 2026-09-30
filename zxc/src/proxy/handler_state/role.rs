use std::fmt::{Display, Formatter};

use crate::file_types::{EXT_WREQ, EXT_WRES, FileType};

pub trait GetRole {
    fn role(&self) -> Role;
}

#[derive(Copy, Clone)]
pub enum Role {
    Client,
    Server,
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Client => write!(f, "client"),
            Role::Server => write!(f, "server"),
        }
    }
}

impl Role {
    pub const fn as_arrow(&self) -> &'static str {
        use Role::*;
        match self {
            Server => "->",
            Client => "<-",
        }
    }

    pub const fn ws_ext(&self) -> &'static str {
        use Role::*;
        match self {
            Server => EXT_WREQ,
            Client => EXT_WRES,
        }
    }

    pub const fn as_ws_ft(&self) -> FileType {
        use Role::*;
        match self {
            Server => FileType::Wreq,
            Client => FileType::Wres,
        }
    }
}
