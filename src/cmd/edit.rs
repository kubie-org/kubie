use std::env;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use which::which;

use crate::cmd::{select_or_list_context, SelectResult};
use crate::kubeconfig;
use crate::settings::Settings;

struct EditorCommand {
    executable: PathBuf,
    args: Vec<String>,
}

impl Display for EditorCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            return write!(f, "{}", self.executable.display());
        }
        write!(f, "{} {}", self.executable.display(), self.args.join(" "))
    }
}

fn parse_editor_command(raw: &str) -> Result<EditorCommand> {
    let mut parts = raw.split_whitespace();
    let executable = parts.next().context("executable is empty")?.into();
    let args: Vec<String> = parts.map(String::from).collect();
    Ok(EditorCommand { executable, args })
}

fn get_editor(settings: &Settings) -> Result<EditorCommand> {
    settings
        .default_editor
        .as_deref()
        .map(|editor| {
            parse_editor_command(editor)
                .with_context(|| format!("unable to parse default_editor command {}", editor))
        })
        .or_else(|| {
            env::var("EDITOR").ok().map(|editor| {
                parse_editor_command(&editor)
                    .with_context(|| format!("unable to parse EDITOR command {}", editor))
            })
        })
        .transpose()?
        .or_else(|| {
            ["nvim", "vim", "emacs", "vi", "nano"].iter().find_map(|editor| {
                which(editor).ok().map(|path| EditorCommand {
                    executable: path,
                    args: vec![],
                })
            })
        })
        .ok_or_else(|| anyhow!("Could not find any editor to use"))
}

pub fn edit_context(settings: &Settings, context_name: Option<String>) -> Result<()> {
    let mut installed = kubeconfig::get_installed_contexts(settings)?;
    installed.contexts.sort_by(|a, b| a.item.name.cmp(&b.item.name));

    let context_name = match context_name {
        Some(context_name) => context_name,
        None => match select_or_list_context(&settings.fzf, &mut installed)? {
            SelectResult::Selected(x) => x,
            _ => return Ok(()),
        },
    };

    let context_src = installed
        .find_context_by_name(&context_name)
        .ok_or_else(|| anyhow!("Could not find context {}", context_name))?;

    let command = get_editor(settings)?;

    let mut job = Command::new(&command.executable)
        .args(&command.args)
        .arg(context_src.source.as_ref())
        .spawn()
        .context(format!("Failed to spawn editor command '{}'", command))?;
    job.wait()?;

    Ok(())
}

pub fn edit_config(settings: &Settings) -> Result<()> {
    let command = get_editor(settings)?;
    let settings_path = Settings::path();

    let mut job = Command::new(&command.executable)
        .args(&command.args)
        .arg(settings_path)
        .spawn()
        .context(format!("Failed to spawn editor command '{}'", command))?;
    job.wait()?;

    Ok(())
}
