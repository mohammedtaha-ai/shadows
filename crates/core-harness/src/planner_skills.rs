//! A Planner session's skills folder: the project's switched-on skills as one
//! plugin, written for one thread and removed when its session goes.

use std::path::Path;

use core_extensions::Item;
use core_threads::ThreadId;

use crate::error::HarnessError;
use crate::profile_home::Homes;
use crate::setup::Setups;

const FOLDER: &str = "planner-skills";
const MANIFEST: &str = concat!(
    r#"{"name":"shadows-skills","version":"0.0.0","#,
    r#""description":"Skills this project gives its Planner"}"#
);

impl Setups {
    /// Writes `skills` to `<root>/planner-skills/<thread>/` as one plugin and
    /// answers its path; with no skill, writes nothing and answers `None`. On
    /// a failure the folder is removed.
    pub(super) fn write_skills(
        &self,
        root: &Path,
        thread: &ThreadId,
        skills: &[Item],
    ) -> Result<Option<String>, HarnessError> {
        if skills.is_empty() {
            return Ok(None);
        }
        let parent = root.join(FOLDER);
        let folder = parent.join(thread.as_str());
        Homes::check_local(&parent)?;
        Homes::check_local(&folder)?;
        let written = (|| -> Result<(), HarnessError> {
            let _ = std::fs::remove_dir_all(&folder);
            let manifest = folder.join(".claude-plugin");
            std::fs::create_dir_all(&manifest).map_err(io)?;
            std::fs::write(manifest.join("plugin.json"), MANIFEST).map_err(io)?;
            for item in skills {
                Homes::copy_item(root, &folder.join("skills").join(&item.name), item)?;
            }
            Ok(())
        })();
        if let Err(error) = written {
            let _ = std::fs::remove_dir_all(&folder);
            return Err(error);
        }
        match folder.to_str() {
            Some(path) => Ok(Some(path.to_string())),
            None => {
                let _ = std::fs::remove_dir_all(&folder);
                Err(HarnessError::ProfileInvalid(
                    "the skills folder path is not valid UTF-8".into(),
                ))
            }
        }
    }

    /// Removes the thread's skills folder, if any. A failure is ignored: the
    /// next opening of the thread removes it again.
    pub(super) fn remove_skills(&self, thread: &ThreadId) {
        if let Some(root) = &self.aylul_root {
            let parent = root.join(FOLDER);
            let folder = parent.join(thread.as_str());
            if Homes::check_local(&parent).is_ok() && Homes::check_local(&folder).is_ok() {
                let _ = std::fs::remove_dir_all(folder);
            }
        }
    }
}

fn io(_: std::io::Error) -> HarnessError {
    HarnessError::ProfileInvalid("cannot write the Planner skills folder".into())
}
