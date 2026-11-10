use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("base64: {0}")]
    Base64(#[from] base64::EncodeSliceError),
    #[error("URL: {0}")]
    Url(#[from] std::str::Utf8Error),
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("base64: {0}")]
    Base64(#[from] base64::DecodeSliceError),
    #[error("URL: {0}")]
    Url(#[from] std::str::Utf8Error),
}

#[derive(Error, Debug)]
pub enum CodecError {
    #[error("encode: {0}")]
    Encode(#[from] EncodeError),
    #[error("decode: {0}")]
    Decode(#[from] DecodeError),
}
