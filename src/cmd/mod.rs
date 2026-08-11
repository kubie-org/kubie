use std::io::{self, IsTerminal};

use anyhow::{bail, Context, Result};

use crate::kubeconfig::Installed;
use crate::kubectl;
use crate::settings::Fzf;

pub mod context;
pub mod delete;
pub mod edit;
pub mod exec;
pub mod export;
pub mod info;
pub mod lint;
pub mod meta;
pub mod namespace;
#[cfg(feature = "update")]
pub mod update;

pub enum SelectResult {
    Cancelled,
    Listed,
    Selected(String),
}

/// The menu needs a terminal on stdin. When stdout is captured, the caller wants the plain list.
fn can_prompt() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

fn context_names(installed: &mut Installed) -> Result<Vec<String>> {
    installed.contexts.sort_by(|a, b| a.item.name.cmp(&b.item.name));
    let context_names: Vec<_> = installed.contexts.iter().map(|c| c.item.name.clone()).collect();

    if context_names.is_empty() {
        bail!("No contexts found");
    }
    Ok(context_names)
}

fn select(fzf: &Fzf, mut items: Vec<String>) -> Result<SelectResult> {
    // NOTE: skim shows the list of items in reverse order
    items.reverse();
    match crate::skim::select(fzf, items)? {
        Some(name) => Ok(SelectResult::Selected(name)),
        None => Ok(SelectResult::Cancelled),
    }
}

pub fn select_or_list_context(fzf: &Fzf, installed: &mut Installed) -> Result<SelectResult> {
    let mut context_names = context_names(installed)?;
    if context_names.len() == 1 {
        return Ok(SelectResult::Selected(context_names.remove(0)));
    }

    if can_prompt() {
        select(fzf, context_names)
    } else {
        for c in context_names {
            println!("{c}");
        }
        Ok(SelectResult::Listed)
    }
}

/// Like `select_or_list_context`, for callers whose output another program reads.
/// It prompts even when stdout is captured, and never prints the list.
pub fn prompt_context(fzf: &Fzf, installed: &mut Installed) -> Result<Option<String>> {
    let mut context_names = context_names(installed)?;
    if context_names.len() == 1 {
        return Ok(Some(context_names.remove(0)));
    }

    if !io::stdin().is_terminal() {
        bail!("No context given and no terminal to prompt on");
    }
    match select(fzf, context_names)? {
        SelectResult::Selected(name) => Ok(Some(name)),
        _ => Ok(None),
    }
}

pub fn select_or_list_namespace(fzf: &Fzf, namespaces: Option<Vec<String>>) -> Result<SelectResult> {
    let mut namespaces = match namespaces {
        Some(ns) => ns,
        None => kubectl::get_namespaces(None).context("Could not get namespaces")?,
    };

    namespaces.sort();

    if namespaces.is_empty() {
        bail!("No namespaces found");
    }

    if can_prompt() {
        select(fzf, namespaces)
    } else {
        for n in namespaces {
            println!("{n}");
        }
        Ok(SelectResult::Listed)
    }
}
