use std::process::Command;

use crate::error::{AppError, Result};

pub fn run_capture(program: &str, args: &[String]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| AppError::Message(format!("failed to start `{program}`: {e}")))?;
    if !output.status.success() {
        return Err(AppError::Process {
            program: program.to_owned(),
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn run_checked(program: &str, args: &[String]) -> Result<()> {
    run_capture(program, args).map(|_| ())
}

pub fn tool_version(program: &str, version_arg: &str) -> Option<String> {
    let output = Command::new(program).arg(version_arg).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_owned())
}
