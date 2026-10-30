use super::{claim, write};

#[test]
fn record_replacement_is_private_without_changing_parent_permissions() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("runtime.json");
    std::fs::write(&path, "stale record").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let _owner = claim(&path).unwrap();
    write(
        &path,
        &serde_json::json!({ "token": "synthetic-capability" }),
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
            "$env:SHADOWS_TEST_RECORD,",
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
            .env("SHADOWS_TEST_RECORD", &path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "the record must grant only its owning user: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(
        std::fs::read_to_string(path)
            .unwrap()
            .contains("synthetic-capability")
    );
}
