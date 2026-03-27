use std::env;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Result};
use which::which;

use crate::cmd::{select_or_list_context, SelectResult};
use crate::kubeconfig;
use crate::settings::Settings;

fn get_editor(settings: &Settings) -> Result<PathBuf> {
    if let Some(default_editor) = &settings.default_editor {
        if let Ok(path) = which(default_editor) {
            return Ok(path);
        }
    }

    env::var("EDITOR")
        .ok()
        .and_then(|editor| which(editor).ok())
        .or_else(|| {
            for editor in &["nvim", "vim", "emacs", "vi", "nano"] {
                if let Ok(path) = which(editor) {
                    return Some(path);
                }
            }
            None
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

    let editor = get_editor(settings)?;

    let mut job = Command::new(editor).arg(context_src.source.as_ref()).spawn()?;
    job.wait()?;

    Ok(())
}

pub fn edit_config(settings: &Settings) -> Result<()> {
    let editor = get_editor(settings)?;
    let settings_path = Settings::path();

    let mut job = Command::new(editor).arg(settings_path).spawn()?;
    job.wait()?;

    Ok(())
}
