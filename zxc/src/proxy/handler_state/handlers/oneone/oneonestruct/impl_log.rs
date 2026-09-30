use std::path::PathBuf;

use header_plz::{
    MessageHead, OneHeader, body_headers::parse::ParseBodyHeaders,
};

use super::*;
use crate::{
    file_types::{EXT_REQ, EXT_RES},
    proxy::handler_state::transition::{Log, UpdateLogExt},
};

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

// Note:
//      history/index - directory created by commander
impl<T, E> UpdateLogExt for OneOneStruct<T, E, OneRequestLine> {
    fn update_extension(&mut self) {
        let path = self.path.as_mut().unwrap();
        path.push(self.log_id.to_string());
        path.set_extension(EXT_REQ);
    }
}

impl<T, E> UpdateLogExt for OneOneStruct<T, E, OneResponseLine> {
    fn update_extension(&mut self) {
        self.path
            .as_mut()
            .unwrap()
            .set_extension(EXT_RES);
    }
}
