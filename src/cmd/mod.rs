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

pub fn select_or_list_context(fzf: &Fzf, installed: &mut Installed) -> Result<SelectResult> {
    installed.contexts.sort_by(|a, b| a.item.name.cmp(&b.item.name));
    let mut context_names: Vec<_> = installed.contexts.iter().map(|c| c.item.name.clone()).collect();

    if context_names.is_empty() {
        bail!("No contexts found");
    }
    if context_names.len() == 1 {
        return Ok(SelectResult::Selected(context_names[0].clone()));
    }

    if io::stdout().is_terminal() {
        // NOTE: skim shows the list of context names in reverse order
        context_names.reverse();
        match crate::skim::select(fzf, context_names)? {
            Some(name) => Ok(SelectResult::Selected(name)),
            None => Ok(SelectResult::Cancelled),
        }
    } else {
        for c in context_names {
            println!("{c}");
        }
        Ok(SelectResult::Listed)
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

    if io::stdout().is_terminal() {
        // NOTE: skim shows the list of namespaces in reverse order
        namespaces.reverse();
        match crate::skim::select(fzf, namespaces)? {
            Some(name) => Ok(SelectResult::Selected(name)),
            None => Ok(SelectResult::Cancelled),
        }
    } else {
        for n in namespaces {
            println!("{n}");
        }
        Ok(SelectResult::Listed)
    }
}
