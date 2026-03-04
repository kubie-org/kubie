use std::io::{self, Cursor, IsTerminal};

use anyhow::{bail, Result};
use skim::prelude::{Key, SkimItemReader};
use skim::{Skim, SkimOptions};

use crate::kubeconfig::Installed;
use crate::kubectl;

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

pub fn select_or_list_context(skim_options: &SkimOptions, installed: &mut Installed) -> Result<SelectResult> {
    installed.contexts.sort_by(|a, b| a.item.name.cmp(&b.item.name));
    let mut context_names: Vec<_> = installed.contexts.iter().map(|c| c.item.name.clone()).collect();

    if context_names.is_empty() {
        bail!("No contexts found");
    }
    if context_names.len() == 1 {
        return Ok(SelectResult::Selected(context_names[0].clone()));
    }

    // Allow interactive selection if stdin OR stderr is a terminal
    if io::stdin().is_terminal() || io::stderr().is_terminal() {
        // NOTE: skim shows the list of context names in reverse order
        context_names.reverse();
        let item_reader = SkimItemReader::default();
        let items = item_reader.of_bufread(Cursor::new(context_names.join("\n")));
        let selected_items = Skim::run_with(skim_options, Some(items))
            .map(|out| match out.final_key {
                Key::Enter => out.selected_items,
                _ => Vec::new(),
            })
            .unwrap_or_default();
        if selected_items.is_empty() {
            return Ok(SelectResult::Cancelled);
        }
        Ok(SelectResult::Selected(selected_items[0].output().to_string()))
    } else {
        for c in context_names {
            println!("{c}");
        }
        Ok(SelectResult::Listed)
    }
}

pub fn select_or_list_namespace(skim_options: &SkimOptions, namespaces: Option<Vec<String>>) -> Result<SelectResult> {
    let mut namespaces = namespaces.unwrap_or_else(|| kubectl::get_namespaces(None).expect("could not get namespaces"));

    namespaces.sort();

    if namespaces.is_empty() {
        bail!("No namespaces found");
    }

    // Allow interactive selection if stdin OR stderr is a terminal
    if io::stdin().is_terminal() || io::stderr().is_terminal() {
        // NOTE: skim shows the list of namespaces in reverse order
        namespaces.reverse();
        let item_reader = SkimItemReader::default();
        let items = item_reader.of_bufread(Cursor::new(namespaces.join("\n")));
        let selected_items = Skim::run_with(skim_options, Some(items))
            .map(|out| match out.final_key {
                Key::Enter => out.selected_items,
                _ => Vec::new(),
            })
            .unwrap_or_default();
        if selected_items.is_empty() {
            return Ok(SelectResult::Cancelled);
        }
        Ok(SelectResult::Selected(selected_items[0].output().to_string()))
    } else {
        for n in namespaces {
            println!("{n}");
        }
        Ok(SelectResult::Listed)
    }
}
