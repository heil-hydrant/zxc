use std::borrow::Cow;
use std::io::Write;

use base64::Engine;
use base64::engine::general_purpose;

use super::Codec;
use super::error::DecodeError;

pub fn perform_decode(
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
    let val: Vec<(Cow<'_, str>, Cow<'_, str>)> =
        form_urlencoded::parse(encoded_str).collect();
    val.iter()
        .fold(Vec::new(), |mut acc: Vec<u8>, (left, right)| {
            let _ = write!(acc, "{}{}", left, right);
            acc
        })
}

fn url_decode_all_characters(
    encoded_str: &[u8],
) -> Result<Vec<u8>, DecodeError> {
    let mut decoded = String::new();
    let mut chars = std::str::from_utf8(encoded_str)?.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            // Attempt to decode the next two characters as hexadecimal
            let hex_chars: String = chars.by_ref().take(2).collect();
            if hex_chars.len() != 2 {
                //return Ok(decoded.into());
                break;
            }

            if let Ok(hex_value) = u8::from_str_radix(&hex_chars, 16) {
                // Successfully decoded a hexadecimal value, append it as a
                // character
                decoded.push(char::from(hex_value));
            } else {
                //return Ok(decoded.into());
                break;
            }
        } else {
            // Not an encoded character, append it as is
            decoded.push(c);
        }
    }
    Ok(decoded.into())
}
