use std::{ffi::OsStr, path::Path, process::{Command, Stdio}};

pub fn execute_sandboxed(
    program: &Path,
    args: &[&OsStr],
) -> std::io::Result<std::process::Child> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    Ok(child)
}
