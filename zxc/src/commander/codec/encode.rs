use base64::Engine;
use base64::engine::general_purpose;
use form_urlencoded::byte_serialize;

use super::Codec;
use super::error::EncodeError;

pub fn perform_encode(
    codec: &Codec,
    data: &[u8],
) -> Result<Vec<u8>, EncodeError> {
    match codec {
        Codec::Base64 => base64_encode(data),
        Codec::Url => Ok(url_encode(data)),
        Codec::UrlAll => url_encode_all_characters(data),
    }
}

fn base64_encode(input: &[u8]) -> Result<Vec<u8>, EncodeError> {
    let mut buf = vec![0; input.len() * 4 / 3 + 4];
    let written = general_purpose::STANDARD.encode_slice(input, &mut buf)?;
    buf.truncate(written);
    Ok(buf)
}

fn url_encode(input: &[u8]) -> Vec<u8> {
    let val = byte_serialize(input)
        .collect::<Vec<&str>>()
        .join("");
    val.into()
}

fn url_encode_all_characters(input: &[u8]) -> Result<Vec<u8>, EncodeError> {
    let mut encoded = String::new();
    for c in std::str::from_utf8(input)?.chars() {
        encoded.push_str(&format!("%{:02X}", c as u32));
    }
    Ok(encoded.into())
}
