use super::SecretKey;
use super::crypto::{Keys, Sealed};

fn keys() -> Keys {
    Keys::derive(&SecretKey::from_bytes([7; 32]))
}

#[test]
fn seal_open_round_trips() {
    let k = keys();
    let sealed = k.aead.seal(b"p\0A", "synthetic").unwrap();
    assert!(
        k.aead.open(b"p\0A", &sealed).unwrap() == "synthetic",
        "value round trips"
    );
}

#[test]
fn open_with_other_aad_fails() {
    let k = keys();
    let sealed = k.aead.seal(b"p\0A", "synthetic").unwrap();
    assert!(k.aead.open(b"p\0B", &sealed).is_err());
}

#[test]
fn open_with_other_key_fails() {
    let k = keys();
    let other = Keys::derive(&SecretKey::from_bytes([8; 32]));
    let sealed = k.aead.seal(b"p\0A", "synthetic").unwrap();
    assert!(other.aead.open(b"p\0A", &sealed).is_err());
}

#[test]
fn short_nonce_is_an_error_not_a_panic() {
    let sealed = Sealed {
        format: 1,
        nonce: vec![0],
        ciphertext: vec![],
    };
    assert!(keys().aead.open(b"p\0A", &sealed).is_err());
}

#[test]
fn unknown_format_is_an_error() {
    let k = keys();
    let mut sealed = k.aead.seal(b"p\0A", "synthetic").unwrap();
    sealed.format = 2;
    assert!(k.aead.open(b"p\0A", &sealed).is_err());
}

#[test]
fn hkdf_keys_differ() {
    let k = keys();
    assert!(k.aead.0 != k.fingerprint);
}
