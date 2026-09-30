use bytes::BytesMut;
use tokio::fs::File;

// Trait to perform file operations
//
// Implemented In: (derive macro)
//     zxc-derive/src/file_ops/mod.rs

pub trait FileOps {
    fn file_and_buf_as_mut(&mut self) -> (&mut File, &mut BytesMut);

    fn attach_file(&mut self, file: File);
}
