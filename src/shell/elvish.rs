use anyhow::Result;
use std::process::Command;

use crate::shell::ShellSpawnInfo;

pub fn spawn_shell(info: &ShellSpawnInfo) -> Result<()> {
    let mut cmd = Command::new("elvish");
    info.env_vars.apply(&mut cmd);
    let _ = cmd.status()?;
    Ok(())
}
