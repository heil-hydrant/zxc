use std::path::PathBuf;

use header_plz::{
    MessageHead, OneHeader, body_headers::parse::ParseBodyHeaders,
};

use super::*;
use crate::proxy::handler_state::transition::write_log::log::{Log, NLog};

impl<T, E, U> Log for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    fn path(&self) -> &PathBuf {
        self.path.as_ref().unwrap() // safe to unwrap
    }

    fn log_data(&self) -> &[u8] {
        self.payload.as_ref().unwrap() // safe to unwrap
    }
}

impl<T, E, U> NLog for OneOneStruct<T, E, U>
where
    U: OneInfoLine + std::fmt::Debug,
    MessageHead<U, OneHeader>: ParseBodyHeaders,
{
    fn log_data(&self) -> impl bytes::Buf {
        self.frame.as_ref().unwrap().as_chain() // safe to unwrap
    }
}
