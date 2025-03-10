use std::path::{Path, PathBuf};

use nu_engine::env::current_dir_str;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::ShellError;

use dialoguer::Input;
use std::error::Error;

#[derive(Default)]
pub struct FileStructure {
    pub resources: Vec<Resource>,
}

impl FileStructure {
    pub fn new() -> FileStructure {
        FileStructure { resources: vec![] }
    }

    pub fn paths_applying_with<F>(
        &mut self,
        to: F,
    ) -> Result<Vec<(PathBuf, PathBuf)>, Box<dyn std::error::Error>>
    where
        F: Fn((PathBuf, usize)) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>>,
    {
        self.resources
            .iter()
            .map(|f| (PathBuf::from(&f.location), f.at))
            .map(to)
            .collect()
    }

    pub fn walk_decorate(
        &mut self,
        start_path: &Path,
        engine_state: &EngineState,
        stack: &Stack,
    ) -> Result<(), ShellError> {
        self.resources = Vec::<Resource>::new();
        self.build(start_path, 0, engine_state, stack)?;
        self.resources.sort();

        Ok(())
    }

    fn build(
        &mut self,
        src: &Path,
        lvl: usize,
        engine_state: &EngineState,
        stack: &Stack,
    ) -> Result<(), ShellError> {
        let source = canonicalize_with(src, current_dir_str(engine_state, stack)?)?;

        if source.is_dir() {
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    self.build(&path, lvl + 1, engine_state, stack)?;
                }

                self.resources.push(Resource {
                    location: path.to_path_buf(),
                    at: lvl,
                });
            }
        } else {
            self.resources.push(Resource {
                location: source,
                at: lvl,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Resource {
    pub at: usize,
    pub location: PathBuf,
}

impl Resource {}

pub fn try_interaction(
    interactive: bool,
    prompt: String,
) -> (Result<Option<bool>, Box<dyn Error>>, bool) {
    let interaction = if interactive {
        match get_interactive_confirmation(prompt) {
            Ok(i) => Ok(Some(i)),
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    };

    let confirmed = match interaction {
        Ok(maybe_input) => maybe_input.unwrap_or(false),
        Err(_) => false,
    };

    (interaction, confirmed)
}

#[allow(dead_code)]
fn get_interactive_confirmation(prompt: String) -> Result<bool, Box<dyn Error>> {
    let input = Input::new()
        .with_prompt(prompt)
        .validate_with(|c_input: &String| -> Result<(), String> {
            if c_input.len() == 1
                && (c_input == "y" || c_input == "Y" || c_input == "n" || c_input == "N")
            {
                Ok(())
            } else if c_input.len() > 1 {
                Err("Enter only one letter (Y/N)".to_string())
            } else {
                Err("Input not valid".to_string())
            }
        })
        .default("Y/N".into())
        .interact_text()?;

    if input == "y" || input == "Y" {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Return `Some(true)` if the last change time of the `src` old than the `dst`,  
/// otherwisie return `Some(false)`. Return `None` if the `src` or `dst` doesn't exist.
pub fn is_older(src: &Path, dst: &Path) -> Option<bool> {
    if !dst.exists() || !src.exists() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let src_ctime = std::fs::metadata(src)
            .map(|m| m.ctime())
            .unwrap_or(i64::MIN);
        let dst_ctime = std::fs::metadata(dst)
            .map(|m| m.ctime())
            .unwrap_or(i64::MAX);
        Some(src_ctime <= dst_ctime)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let src_ctime = std::fs::metadata(src)
            .map(|m| m.last_write_time())
            .unwrap_or(u64::MIN);
        let dst_ctime = std::fs::metadata(dst)
            .map(|m| m.last_write_time())
            .unwrap_or(u64::MAX);
        Some(src_ctime <= dst_ctime)
    }
}

#[cfg(unix)]
pub mod users {
    use libc::{c_int, gid_t, uid_t};
    use nix::unistd::{Gid, Group, Uid, User};
    use std::ffi::CString;

    pub fn get_user_by_uid(uid: uid_t) -> Option<User> {
        User::from_uid(Uid::from_raw(uid)).ok().flatten()
    }

    pub fn get_group_by_gid(gid: gid_t) -> Option<Group> {
        Group::from_gid(Gid::from_raw(gid)).ok().flatten()
    }

    pub fn get_current_uid() -> uid_t {
        Uid::current().as_raw()
    }

    pub fn get_current_gid() -> gid_t {
        Gid::current().as_raw()
    }

    pub fn get_current_username() -> Option<String> {
        User::from_uid(Uid::current())
            .ok()
            .flatten()
            .map(|user| user.name)
    }

    /// Returns groups for a provided user name and primary group id.
    ///
    /// # libc functions used
    ///
    /// - [`getgrouplist`](https://docs.rs/libc/*/libc/fn.getgrouplist.html)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use users::get_user_groups;
    ///
    /// for group in get_user_groups("stevedore", 1001).expect("Error looking up groups") {
    ///     println!("User is a member of group #{group}");
    /// }
    /// ```
    pub fn get_user_groups(username: &str, gid: gid_t) -> Option<Vec<Gid>> {
        // MacOS uses i32 instead of gid_t in getgrouplist for unknown reasons
        #[cfg(target_os = "macos")]
        let mut buff: Vec<i32> = vec![0; 1024];
        #[cfg(not(target_os = "macos"))]
        let mut buff: Vec<gid_t> = vec![0; 1024];

        let Ok(name) = CString::new(username.as_bytes()) else {
            return None;
        };

        let mut count = buff.len() as c_int;

        // MacOS uses i32 instead of gid_t in getgrouplist for unknown reasons
        // SAFETY:
        // int getgrouplist(const char *user, gid_t group, gid_t *groups, int *ngroups);
        //
        // `name` is valid CStr to be `const char*` for `user`
        // every valid value will be accepted for `group`
        // The capacity for `*groups` is passed in as `*ngroups` which is the buffer max length/capacity (as we initialize with 0)
        // Following reads from `*groups`/`buff` will only happen after `buff.truncate(*ngroups)`
        #[cfg(target_os = "macos")]
        let res =
            unsafe { libc::getgrouplist(name.as_ptr(), gid as i32, buff.as_mut_ptr(), &mut count) };

        #[cfg(not(target_os = "macos"))]
        let res = unsafe { libc::getgrouplist(name.as_ptr(), gid, buff.as_mut_ptr(), &mut count) };

        if res < 0 {
            None
        } else {
            buff.truncate(count as usize);
            buff.sort_unstable();
            buff.dedup();
            // allow trivial cast: on macos i is i32, on linux it's already gid_t
            #[allow(trivial_numeric_casts)]
            buff.into_iter()
                .filter_map(|i| get_group_by_gid(i as gid_t))
                .map(|group| group.gid)
                .collect::<Vec<_>>()
                .into()
        }
    }
}
