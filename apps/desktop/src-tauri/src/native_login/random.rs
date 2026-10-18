//! The OS-preferred random generator behind the verifier and the pending store file name.
use std::{io, ptr};
use windows_sys::Win32::Security::Cryptography::{
    BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
};

pub(super) fn bytes() -> io::Result<[u8; 32]> {
    let mut bytes = [0; 32];
    let status = unsafe {
        BCryptGenRandom(
            ptr::null_mut(),
            bytes.as_mut_ptr(),
            32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        return Err(io::Error::other("OS random generator failed"));
    }
    Ok(bytes)
}
