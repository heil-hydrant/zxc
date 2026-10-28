mod convert;
mod pipeline;

use std::path::PathBuf;

use bytes::BytesMut;
use tokio::fs::File;
use zxc_derive::RepeaterReadFile;

use crate::repeater::prelude::*;

// repeater http handler struct
#[derive(RepeaterReadFile)]
pub struct Roneone<T> {
    buf: BytesMut,
    path: PathBuf,
    stream: T,
    file: File,
    payload: Option<BytesMut>,
    update: bool,
}

impl<T> Roneone<T> {
    pub fn get_payload(&mut self) -> Option<BytesMut> {
        self.payload.take()
    }
}
