use std::fs::read_to_string;

use rustls_pemfile::{Item, read_one_from_slice};
use rustls_pki_types::PrivateKeyDer;

use super::CryptoBuildError;
use crate::config::global::parser::global_config_path;

// https://github.com/rustls/pemfile/issues/39

// Convert &str (PRIVATE_KEY) to PrivateKeyDer
pub fn str_to_private(
    pk: &str,
) -> Result<PrivateKeyDer<'static>, CryptoBuildError> {
    let (new_key, _) = read_one_from_slice(pk.as_bytes())
        .map_err(|e| CryptoBuildError::ReadPrivateKey(format!("{e:?}")))?
        .ok_or(CryptoBuildError::NoKeyFound)?;
    if let Item::Pkcs8Key(val) = new_key {
        Ok(val.into())
    } else {
        Err(CryptoBuildError::UnknownPrivateKeyType)
    }
}

// private key - $HOME/.config/zxc/private.key
pub fn read_private() -> Result<String, CryptoBuildError> {
    let mut path = global_config_path()?;
    path.push("private.key");
    Ok(read_to_string(path)?)
}
