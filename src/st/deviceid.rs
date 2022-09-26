use crate::st::{error::Error, luhn::*};
use base32::{encode, Alphabet};
use itertools::Itertools;
use openssl::x509::X509;
use sha2::{Digest, Sha256};

/// Generate device id from pem cert.
pub fn get_device_id(cert: &str) -> Result<String, Error> {
    // convert to der, then calculte sha256
    let cert = X509::from_pem(&cert.as_bytes())?.to_der()?;
    let mut hasher = Sha256::new();
    hasher.update(cert);
    // base32 encode sha hash and remove trailing "="
    let id = encode(Alphabet::RFC4648 { padding: true }, &hasher.finalize())
        .trim_end_matches('=')
        .to_string();
    let luhn = luhnify(&id)?;
    let mut result = String::new();
    // This replaces `chunkify` in main/lib/protocol/deviceid.go, add "-" every 7th character
    luhn.chars().chunks(7).into_iter().enumerate().for_each(|(i, chunk)| {
        if i > 0 {
            result.push('-');
        }
        result.push_str(&chunk.into_iter().collect::<String>());
    });
    Ok(result)
}

/// translated from main/lib/protocol/deviceid.go
pub fn luhnify(s: &str) -> Result<String, Error> {
    let mut result = String::new();
    // break into 13 character chunks and calculate check bit for each chunk
    for chunk in s.chars().chunks(13).into_iter() {
        let string: String = chunk.into_iter().collect();
        result.push_str(&string);
        result.push(luhn32(&string)?);
    }
    Ok(result)
}
