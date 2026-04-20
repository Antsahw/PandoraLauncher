#![deny(unused_must_use)]

use std::{ffi::OsStr, path::Path, sync::Arc};

#[cfg(target_os = "linux")]
mod linux;

mod path_cache;

pub use path_cache::{get_command_path, is_command_available};

pub fn execute_with_sandbox(
    program: &Path,
    args: &[&OsStr],
) -> std::io::Result<std::process::Child> {
    #[cfg(target_os = "linux")]
    {
        linux::execute_sandboxed(program, args)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut cmd = std::process::Command::new(program);
        cmd.args(args);
        cmd.spawn()
    }
}
