use std::{ffi::OsStr, path::{Path, PathBuf}, sync::Arc};

pub fn get_command_path(command: &OsStr) -> Option<Arc<Path>> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(command);
            if candidate.is_file() {
                return Some(Arc::from(candidate));
            }
        }
    }
    None
}

pub fn is_command_available(command: &'static str) -> bool {
    get_command_path(OsStr::new(command)).is_some()
}
