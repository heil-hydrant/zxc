pub mod decode;
pub mod encode;
pub mod error;

pub use error::CodecError;
use serde::Deserialize;
use serde_json::{Value, json};

use self::decode::perform_decode;
use self::encode::perform_encode;

// Enum to represent different codecs
// derive copy clone only for testing
#[cfg_attr(test, derive(Clone, Copy))]
#[derive(Debug, Deserialize)]
pub enum Codec {
    Base64,
    Url,
    UrlAll,
}

pub fn perform_codec_op(
    encode: bool,
    codec: &Codec,
    data: &[u8],
) -> Result<Value, CodecError> {
    let result = if encode {
        perform_encode(codec, data)?
    } else {
        perform_decode(codec, data)?
    };
    Ok(json!({"result" : String::from_utf8_lossy(&result)}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64() {
        let codec = Codec::Base64;
        let data = b"hello world";
        let verify_encoded = "aGVsbG8gd29ybGQ=";
        let encoded = encode::perform_encode(&codec, data).unwrap();
        assert_eq!(encoded, verify_encoded.as_bytes());
        let decoded = decode::perform_decode(&codec, &encoded).unwrap();
        assert_eq!(data, decoded.as_slice());
    }

    #[test]
    fn test_url() {
        let codec = Codec::Url;
        let data = b"hello world";
        let verify_encoded = "hello+world";
        let encoded = encode::perform_encode(&codec, data).unwrap();
        assert_eq!(encoded, verify_encoded.as_bytes());
        let decoded = decode::perform_decode(&codec, &encoded).unwrap();
        assert_eq!(data, decoded.as_slice());
    }

    #[test]
    fn test_url_all() {
        let codec = Codec::UrlAll;
        let data = b"hello world";
        let verify_encoded = "%68%65%6C%6C%6F%20%77%6F%72%6C%64";
        let encoded = encode::perform_encode(&codec, data).unwrap();
        assert_eq!(encoded, verify_encoded.as_bytes());
        let decoded = decode::perform_decode(&codec, &encoded).unwrap();
        assert_eq!(data, decoded.as_slice());
    }
}
