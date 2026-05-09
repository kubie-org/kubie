use std::process::Command;
use std::str;

use anyhow::{anyhow, Context, Result};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
    Elvish,
    Fish,
    Xonsh,
    Zsh,
    Nu,
}

impl ShellKind {
    pub fn from_str(name: &str) -> Option<ShellKind> {
        Some(match name {
            "bash" | "dash" => ShellKind::Bash,
            "elvish" => ShellKind::Elvish,
            "fish" => ShellKind::Fish,
            "xonsh" | "python" => ShellKind::Xonsh,
            "zsh" => ShellKind::Zsh,
            "nu" => ShellKind::Nu,
            _ => return None,
        })
    }
}

fn run_ps(args: &[&str]) -> Result<Vec<String>> {
    let result = Command::new("ps").args(args).output().context("Could not spawn ps")?;

    if !result.status.success() {
        let stderr = str::from_utf8(&result.stderr).unwrap_or("Could not decode stderr of ps as utf-8");
        return Err(anyhow!("Error calling ps: {}", stderr));
    }

    let text = str::from_utf8(&result.stdout)?;
    Ok(text.split('\n').filter(|x| !x.is_empty()).map(String::from).collect())
}

fn parent_of(pid: &str) -> Result<String> {
    let lines = run_ps(&["-o", "ppid=", pid])?;
    lines
        .into_iter()
        .next()
        .map(|x| x.trim().to_string())
        .ok_or_else(|| anyhow!("Could not get parent pid of pid={}", pid))
}

fn command_of(pid: &str) -> Result<String> {
    let lines = run_ps(&["-o", "args=", pid])?;
    lines
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Could not get command of pid={}", pid))
}

fn parse_command(cmd: &str) -> &str {
    let first_space = cmd.find(' ').unwrap_or(cmd.len());
    let binary_path = &cmd[..first_space];
    let last_path_sep = binary_path.rfind('/').map(|x| x + 1).unwrap_or(0);
    let binary = &binary_path[last_path_sep..];
    binary
        .trim_start_matches('-')
        .trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')
}

/// Detect from which kind of shell kubie was spawned.
///
/// This function walks up the process tree and finds all the ancestors to kubie.
/// If any of kubie's ancestor is a known shell, we have found which shell is in
/// use.
///
/// This functions depends on the `ps` command being installed and available in
/// the PATH variable.
///
/// The SHELL environment variable corresponds to the user's configured SHELL, not
/// the shell currently in use.
pub fn detect() -> Result<ShellKind> {
    let kubie_pid = format!("{}", std::process::id());
    let mut parent_pid = parent_of(&kubie_pid)?;
    loop {
        if parent_pid == "1" {
            return Err(anyhow!("Could not detect shell in use"));
        }

        let cmd = command_of(&parent_pid)?;
        let name = parse_command(&cmd);
        if let Some(kind) = ShellKind::from_str(name) {
            return Ok(kind);
        }

        parent_pid = parent_of(&parent_pid)?;
    }
}

#[test]
fn test_parse_command_simple() {
    assert_eq!(parse_command("bash"), "bash");
}

#[test]
fn test_parse_command_with_args() {
    assert_eq!(parse_command("bash --rcfile hello.sh"), "bash");
}

#[test]
fn test_parse_command_with_path() {
    assert_eq!(parse_command("/bin/bash"), "bash");
}

#[test]
fn test_parse_command_with_path_and_args() {
    assert_eq!(parse_command("/bin/bash --rcfile hello.sh"), "bash");
}

#[test]
fn test_parse_command_login_shell() {
    assert_eq!(parse_command("-zsh"), "zsh");
}

#[test]
fn test_parse_command_versioned_intepreter() {
    assert_eq!(parse_command("python3.8"), "python");
}

#[test]
fn test_parse_command_nu() {
    assert_eq!(parse_command("/bin/nu"), "nu");
}
