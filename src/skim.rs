use std::io::Cursor;

use anyhow::Result;
use skim::prelude::{SkimItemReader, SkimOptionsBuilder};
use skim::Skim;

use crate::settings::Fzf;

fn build_options(fzf: &Fzf) -> Result<skim::SkimOptions> {
    let mut options = SkimOptionsBuilder::default();

    options
        .no_multi(true)
        .no_mouse(!fzf.mouse)
        .reverse(fzf.reverse);

    if let Some(color) = &fzf.color {
        options.color(color.clone());
    }

    if fzf.ignore_case {
        options.case(skim::CaseMatching::Ignore);
    }

    if fzf.info_hidden {
        options.no_info(true);
    }

    if let Some(height) = &fzf.height {
        options.height(height.clone());
    }

    if let Some(prompt) = &fzf.prompt {
        options.prompt(prompt.clone());
    }

    options
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build skim options: {}", e))
}

/// Run skim with the given items and return the selected item, if any
pub fn select(fzf: &Fzf, items: Vec<String>) -> Result<Option<String>> {
    let options = build_options(fzf)?;
    let reader = SkimItemReader::default();
    let rx = reader.of_bufread(Cursor::new(items.join("\n")));
    let output = Skim::run_with(options, Some(rx)).map_err(|e| anyhow::anyhow!("{e}"))?;

    if output.is_abort || output.selected_items.is_empty() {
        Ok(None)
    } else {
        Ok(Some(output.selected_items[0].output().to_string()))
    }
}
