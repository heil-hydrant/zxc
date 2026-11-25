use base64::Engine;
use base64::engine::general_purpose;

use super::Codec;
use super::error::DecodeError;

pub(super) fn perform_decode(
    codec: &Codec,
    data: &[u8],
) -> Result<Vec<u8>, DecodeError> {
    match codec {
        Codec::Base64 => base64_decode(data),
        Codec::Url => Ok(url_decode(data)),
        Codec::UrlAll => url_decode_all_characters(data),
    }
}

fn base64_decode(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut buf = vec![0; data.len() * 3 / 4 + 4];
    let written = general_purpose::STANDARD.decode_slice(data, &mut buf)?;
    buf.truncate(written);
    Ok(buf)
}

fn url_decode(encoded_str: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(encoded_str.len());
    let mut i = 0;
    while i < encoded_str.len() {
        match encoded_str[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < encoded_str.len() => {
                if let Ok(hex) =
                    std::str::from_utf8(&encoded_str[i + 1..i + 3])
                    && let Ok(val) = u8::from_str_radix(hex, 16)
                {
                    out.push(val);
                    i += 3;
                    continue;
                }
                out.push(encoded_str[i]);
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    out
}

fn url_decode_all_characters(
    encoded_str: &[u8],
) -> Result<Vec<u8>, DecodeError> {
    let _ = std::str::from_utf8(encoded_str)?;
    let mut decoded = Vec::with_capacity(encoded_str.len());
    let mut i = 0;
    while i < encoded_str.len() {
        if encoded_str[i] == b'%'
            && i + 2 < encoded_str.len()
            && let Ok(hex) = std::str::from_utf8(&encoded_str[i + 1..i + 3])
            && let Ok(val) = u8::from_str_radix(hex, 16)
        {
            decoded.push(val);
            i += 3;
            continue;
        }
        decoded.push(encoded_str[i]);
        i += 1;
    }
    Ok(decoded)
}
