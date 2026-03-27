use anyhow::Result;

use crate::cmd::{select_or_list_context, SelectResult};
use crate::kubeconfig;
use crate::settings::Settings;

pub fn delete_context(settings: &Settings, context_name: Option<String>) -> Result<()> {
    let mut installed = kubeconfig::get_installed_contexts(settings)?;

    let context_name = match context_name {
        Some(context_name) => context_name,
        None => match select_or_list_context(&settings.fzf, &mut installed)? {
            SelectResult::Selected(x) => x,
            _ => return Ok(()),
        },
    };

    installed.delete_context(&context_name)
}
