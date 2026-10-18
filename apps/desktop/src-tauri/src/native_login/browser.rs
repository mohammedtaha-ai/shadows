//! Launch the system browser, only at the reviewed backend origin.
use std::{io, ptr};

/// Caller supplies the reviewed HTTPS origin from backend configuration. Not IPC.
pub fn open_system_browser(url: &url::Url, reviewed_origin: &url::Url) -> io::Result<()> {
    if url.scheme() != "https"
        || url.origin() != reviewed_origin.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unreviewed browser origin",
        ));
    }
    let value: Vec<u16> = url.as_str().encode_utf16().chain(Some(0)).collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            value.as_ptr(),
            ptr::null(),
            ptr::null(),
            1,
        )
    };
    if result as isize <= 32 {
        return Err(io::Error::other("system browser launch failed"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_refuses_unreviewed_or_unsafe_url_before_launch() {
        let origin = url::Url::parse("https://backend.example").unwrap();
        for value in [
            "http://backend.example/login",
            "https://foreign.example/login",
            "https://user:pass@backend.example/login",
            "https://backend.example/login#fragment",
        ] {
            assert!(open_system_browser(&url::Url::parse(value).unwrap(), &origin).is_err());
        }
    }
}
