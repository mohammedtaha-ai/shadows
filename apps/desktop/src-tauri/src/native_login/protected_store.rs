//! Persist opaque material under Windows user-scoped DPAPI. Loading never establishes identity.
use super::random;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::{
    io::{self, Read, Write},
    path::Path,
    ptr,
};
use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};

fn crypt(value: &[u8], decrypt: bool) -> io::Result<Vec<u8>> {
    if value.is_empty() || value.len() > 65536 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid protected material size",
        ));
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: value.len() as u32,
        pbData: value.as_ptr().cast_mut(),
    };
    let mut out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    let ok = unsafe {
        if decrypt {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        } else {
            CryptProtectData(
                &input,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        }
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let bytes = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe { LocalFree(out.pbData.cast()) };
    Ok(bytes)
}
fn protect(value: &[u8]) -> io::Result<Vec<u8>> {
    crypt(value, false)
}
fn unprotect(value: &[u8]) -> io::Result<Vec<u8>> {
    crypt(value, true)
}
pub fn save_session(path: &Path, handle: &[u8]) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let encrypted = protect(handle)?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("session path requires parent"))?;
    std::fs::create_dir_all(parent)?;
    let pending = parent.join(format!(
        ".session-{}.pending",
        URL_SAFE_NO_PAD.encode(random::bytes()?)
    ));
    let result = (|| {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        f.write_all(&encrypted)?;
        f.sync_all()?;
        drop(f);
        let from: Vec<_> = pending.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&pending);
    }
    result
}
pub fn load_session(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut blob = Vec::new();
    f.take(65537).read_to_end(&mut blob)?;
    unprotect(&blob).map(Some)
}
pub fn clear_session(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        v => v,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dpapi_roundtrip_and_tamper_refusal() {
        let plain = b"fixture-only-opaque-session";
        let mut blob = protect(plain).unwrap();
        assert!(!blob.windows(plain.len()).any(|w| w == plain));
        assert_eq!(unprotect(&blob).unwrap(), plain);
        let n = blob.len();
        assert!(n > 0);
        blob[n / 2] ^= 1;
        assert!(unprotect(&blob).is_err());
    }
    #[test]
    fn restart_load_then_local_logout_clears_material() {
        let path =
            std::env::temp_dir().join(format!("nylra-native-test-{}.bin", std::process::id()));
        clear_session(&path).unwrap();
        save_session(&path, b"fixture-only-opaque-session").unwrap();
        assert_eq!(
            load_session(&path).unwrap().unwrap(),
            b"fixture-only-opaque-session"
        );
        clear_session(&path).unwrap();
        assert!(load_session(&path).unwrap().is_none());
        clear_session(&path).unwrap();
    }
    #[test]
    fn protected_store_replaces_material_and_refuses_corruption() {
        let dir = std::env::temp_dir().join(format!("nylra-replace-test-{}", std::process::id()));
        let path = dir.join("session.bin");
        save_session(&path, b"fixture-first").unwrap();
        save_session(&path, b"fixture-second").unwrap();
        assert_eq!(load_session(&path).unwrap().unwrap(), b"fixture-second");
        std::fs::write(&path, b"corrupt").unwrap();
        assert!(load_session(&path).is_err());
        clear_session(&path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
