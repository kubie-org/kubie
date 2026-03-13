use std::io::{BufWriter, Write};
use std::process::Command;

use anyhow::Result;

use super::ShellSpawnInfo;

pub fn spawn_shell(info: &ShellSpawnInfo) -> Result<()> {
    let temp_rc_file = tempfile::Builder::new()
        .prefix("kubie-xonshrc")
        .suffix(".xsh")
        .tempfile()?;
    let mut temp_rc_file_buf = BufWriter::new(temp_rc_file.as_file());

    write!(
        temp_rc_file_buf,
        r#"
# https://xon.sh/xonshrc.html
from pathlib import Path

files = $XONSHRC

for file in files:
    if Path(file).is_file():
        source @(file)
if Path("~/.config/xonsh/rc.d").is_dir():
    for file in path.glob('*.xsh'):
        source @(file)

@events.on_precommand
def __kubie_cmd_pre_exec__(cmd):
    $KUBECONFIG = $KUBIE_KUBECONFIG
"#
    )?;

    if !info.settings.prompt.disable {
        write!(
            temp_rc_file_buf,
            r#"
$KUBIE_PROMPT='{}'
import re

# Fanciful prompt-command replacement as xonsh forces the use of PROMPT_FIELDS
for match in re.finditer(r'\$\(([^)]*)\)', $KUBIE_PROMPT):
    command = match.group(1)
    name = command.split().pop()
    $PROMPT_FIELDS[name] = evalx(f'lambda: $({{command}}).strip()')
    $KUBIE_PROMPT = $KUBIE_PROMPT.replace(f'$({{command}})', '{{' + name + '}}')

if $KUBIE_XONSH_USE_RIGHT_PROMPT == "1":
    $RIGHT_PROMPT = $KUBIE_PROMPT + $RIGHT_PROMPT
else:
    $PROMPT = $KUBIE_PROMPT + $PROMPT

del $KUBIE_PROMPT
"#,
            info.prompt,
        )?;
    }

    temp_rc_file_buf.flush()?;

    let mut cmd = Command::new("xonsh");
    cmd.arg("--rc");
    cmd.arg(temp_rc_file.path());
    info.env_vars.apply(&mut cmd);

    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok(())
}
