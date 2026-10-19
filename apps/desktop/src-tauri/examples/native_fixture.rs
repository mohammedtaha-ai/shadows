//! Synthetic Windows plumbing only, NOT the backend login contract or OIDC.
//! Run each store operation in a fresh process to prove process restart.
use nylra_desktop::native_login::{self, ReturnBinding};
use std::{io, path::PathBuf, time::Duration};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let action = args.get(1).map(String::as_str).unwrap_or("");
    match action {
        "browser" => {
            let binding = ReturnBinding::new()?;
            let url = format!("{}?ticket=fixture-only", binding.return_uri());
            // Test utility deliberately launches the owned HTTP listener, whereas
            // production open_system_browser only accepts a reviewed HTTPS origin.
            let value: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
            let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
            let launched = unsafe {
                windows_sys::Win32::UI::Shell::ShellExecuteW(
                    std::ptr::null_mut(),
                    verb.as_ptr(),
                    value.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1,
                )
            };
            if launched as isize <= 32 {
                return Err(io::Error::other("fixture system browser launch failed"));
            }
            let returned = binding.wait(Duration::from_secs(30))?;
            assert_eq!(returned.callback.query(), Some("ticket=fixture-only"));
            assert_eq!(returned.verifier().len(), 43);
            println!("PASS fixture: system browser reached owned native listener; no backend login occurred");
        }
        "save" | "restart" | "logout" | "cleared" => {
            let dir = PathBuf::from(args.get(2).expect("fixture output directory required"));
            let path = dir.join("fixture-session.bin");
            match action {
                "save" => native_login::save_session(&path, b"fixture-only-session")?,
                "restart" => assert_eq!(
                    native_login::load_session(&path)?.unwrap(),
                    b"fixture-only-session"
                ),
                "logout" => native_login::clear_session(&path)?,
                "cleared" => assert!(native_login::load_session(&path)?.is_none()),
                _ => unreachable!(),
            }
            println!("PASS fixture: {action}; no authentication or remote revocation claim");
        }
        _ => {
            return Err(io::Error::other(
                "expected browser/save/restart/logout/cleared",
            ))
        }
    }
    Ok(())
}
