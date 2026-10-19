//! Exclusive ownership of a private local connection descriptor.
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub origin: String,
    pub installation_id: String,
    pub token: String,
}

pub fn claim(path: &Path) -> anyhow::Result<File> {
    if !path.is_absolute() {
        anyhow::bail!("descriptor path must be absolute");
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid descriptor directory"))?;
    std::fs::create_dir_all(parent)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path.with_extension("owner"))?;
    file.try_lock()
        .map_err(|_| anyhow::anyhow!("product daemon already owns this descriptor"))?;
    Ok(file)
}

pub fn write(path: &Path, descriptor: &Descriptor) -> anyhow::Result<()> {
    // A fresh inode avoids inheriting an old descriptor's broad permissions or
    // following its symlink. Protect the empty file before writing capability bytes.
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = write_private(&temporary, descriptor).and_then(|()| {
        std::fs::rename(&temporary, path)?;
        Ok(())
    });
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

fn write_private(path: &Path, descriptor: &Descriptor) -> anyhow::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // GENERIC_WRITE plus WRITE_DAC for owner-only protection on the handle.
        options.access_mode(0x4000_0000 | 0x0004_0000);
    }
    let mut file = options.open(path)?;
    #[cfg(windows)]
    protect_windows(&file)?;
    file.write_all(&serde_json::to_vec(descriptor)?)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn protect_windows(file: &File) -> std::io::Result<()> {
    use std::{os::windows::io::AsRawHandle, ptr};
    use windows_sys::Win32::{
        Foundation::{HANDLE, LocalFree},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SE_FILE_OBJECT, SetSecurityInfo,
            },
            DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetTokenInformation,
            PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_USER, TokenUser,
        },
    };
    // SAFETY: -4 is Windows' documented current-process-token pseudo handle.
    // Token and security descriptor buffers outlive each API use; LocalFree
    // releases only successful API allocations. No secret is passed to Windows.
    unsafe {
        let token = -4isize as HANDLE;
        let mut needed = 0;
        GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut needed);
        let mut buffer = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
        if GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let mut sid = ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut length = 0;
        while *sid.add(length) != 0 {
            length += 1;
        }
        let sid_text = String::from_utf16_lossy(std::slice::from_raw_parts(sid, length));
        LocalFree(sid.cast());
        let sddl: Vec<u16> = format!("D:P(A;;FA;;;{sid_text})")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut descriptor = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let (mut present, mut defaulted, mut acl) = (0, 0, ptr::null_mut());
        let result =
            if GetSecurityDescriptorDacl(descriptor, &mut present, &mut acl, &mut defaulted) == 0
                || present == 0
            {
                Err(std::io::Error::last_os_error())
            } else {
                let status = SetSecurityInfo(
                    file.as_raw_handle(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    acl,
                    ptr::null(),
                );
                if status == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::from_raw_os_error(status as i32))
                }
            };
        LocalFree(descriptor);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_replacement_is_private_without_changing_parent_permissions() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("connection.json");
        std::fs::write(&path, "stale descriptor").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let _owner = claim(&path).unwrap();
        write(
            &path,
            &Descriptor {
                origin: "http://127.0.0.1:4320".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
                token: "synthetic-capability".into(),
            },
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(root.path()).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let check = concat!(
                "$acl=[Security.AccessControl.FileSecurity]::new(",
                "$env:SHADOWS_TEST_DESCRIPTOR,",
                "[Security.AccessControl.AccessControlSections]::Access); ",
                "$rules=$acl.GetAccessRules($true,$false,",
                "[Security.Principal.SecurityIdentifier]); ",
                "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value; ",
                "if(-not $acl.AreAccessRulesProtected -or $rules.Count -ne 1 ",
                "-or $rules[0].IdentityReference.Value -ne $sid){exit 1}",
            );
            let output = std::process::Command::new("powershell.exe")
                .creation_flags(0x0800_0000)
                .args(["-NoProfile", "-NonInteractive", "-Command", check])
                .env("SHADOWS_TEST_DESCRIPTOR", &path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "descriptor must grant only its owning user: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains("synthetic-capability")
        );
    }
}
