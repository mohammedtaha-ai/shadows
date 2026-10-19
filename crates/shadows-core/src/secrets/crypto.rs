//! AEAD envelopes bound to a profile/name, with independent fingerprint keys.
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand::{RngCore, rngs::OsRng};
use sha2::Sha256;

use super::SecretKey;

pub(super) struct AeadKey(pub(super) [u8; 32]);

pub(super) struct Keys {
    pub(super) aead: AeadKey,
    pub(super) fingerprint: [u8; 32],
}

pub(super) struct Sealed {
    pub(super) format: i64,
    pub(super) nonce: Vec<u8>,
    pub(super) ciphertext: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct CryptoError;

impl Keys {
    pub(super) fn derive(master: &SecretKey) -> Self {
        let hkdf = Hkdf::<Sha256>::new(None, &master.0);
        let mut aead = [0; 32];
        let mut fingerprint = [0; 32];
        hkdf.expand(b"shadows/secrets/aead/v1", &mut aead)
            .expect("32 bytes fit HKDF-SHA256");
        hkdf.expand(b"shadows/secrets/fingerprint/v1", &mut fingerprint)
            .expect("32 bytes fit HKDF-SHA256");
        Self {
            aead: AeadKey(aead),
            fingerprint,
        }
    }

    pub(super) fn value_mac(&self, value: &str) -> String {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.fingerprint)
            .expect("HMAC accepts 32 bytes");
        mac.update(value.as_bytes());
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

pub(super) fn seal(key: &AeadKey, aad: &[u8], plain: &str) -> Result<Sealed, CryptoError> {
    let mut nonce = [0; 12];
    OsRng.try_fill_bytes(&mut nonce).map_err(|_| CryptoError)?;
    let cipher = Aes256Gcm::new((&key.0).into());
    let ciphertext = cipher
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: plain.as_bytes(),
                aad,
            },
        )
        .map_err(|_| CryptoError)?;
    Ok(Sealed {
        format: 1,
        nonce: nonce.to_vec(),
        ciphertext,
    })
}

pub(super) fn open(key: &AeadKey, aad: &[u8], sealed: &Sealed) -> Result<String, CryptoError> {
    if sealed.format != 1 || sealed.nonce.len() != 12 {
        return Err(CryptoError);
    }
    let cipher = Aes256Gcm::new((&key.0).into());
    let nonce: [u8; 12] = sealed
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError)?;
    let bytes = cipher
        .decrypt(
            &Nonce::from(nonce),
            Payload {
                msg: &sealed.ciphertext,
                aad,
            },
        )
        .map_err(|_| CryptoError)?;
    String::from_utf8(bytes).map_err(|_| CryptoError)
}
