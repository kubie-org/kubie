use anyhow::Result;
use skim::prelude::SkimOptionsBuilder;

use crate::settings::Fzf;

pub fn build_options(fzf: &Fzf) -> Result<skim::SkimOptions> {
    let mut options = SkimOptionsBuilder::default();

    options
        .no_multi(true)
        .no_mouse(!fzf.mouse)
        .reverse(fzf.reverse)
        .color(fzf.color.clone());

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
