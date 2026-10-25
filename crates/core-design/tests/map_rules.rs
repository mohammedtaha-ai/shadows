use core_design::{PartContent, TestMapPart as MapPart, TestMapRules as MapRules, VisionContent};
use core_standards::EffectiveStandards;

#[test]
fn windows_device_names_are_refused() {
    for path in ["nul", "backend/con.txt", "Com1", "LPT9.rs"] {
        assert!(MapRules::normalise_folder(path).is_err(), "{path}");
    }
}

#[test]
fn reserved_initial_files_are_refused() {
    for path in ["backend/CONTRACT.yaml", "SHADOWS.md"] {
        let content = PartContent {
            title: "Backend".into(),
            folders: vec!["backend".into()],
            files: vec![core_design::InitialFile {
                path: path.into(),
                content: "".into(),
            }],
            ..Default::default()
        };
        let parts = [MapPart {
            key: "p1",
            parent: None,
            content: &content,
        }];
        let standards = EffectiveStandards {
            base: core_standards::Standards::base().clone(),
            additions: None,
        };
        assert_eq!(
            MapRules::check_shape(&parts, &["p1"], &VisionContent::default(), &standards),
            Err(format!("reserved path: {path}"))
        );
        // An unchanged part's reserved file never blocks another part's
        // change: each can then be repaired one at a time. (A root
        // SHADOWS.md is also outside its part's folders, refused anyway.)
        if path != "SHADOWS.md" {
            assert_eq!(
                MapRules::check_shape(&parts, &[], &VisionContent::default(), &standards),
                Ok(())
            );
        }
    }
}
