use anyhow::{bail, Result};

use crate::cmd::prompt_context;
use crate::kubeconfig::{self, Installed, KubeConfig};
use crate::settings::Settings;
use crate::state::State;

/// Print a kubeconfig path per matched context. Nothing else goes to stdout.
pub fn export(settings: &Settings, context_name: Option<String>, namespace_name: Option<String>) -> Result<()> {
    let mut installed = kubeconfig::get_installed_contexts(settings)?;

    let pattern = match context_name {
        Some(name) => name,
        None => match prompt_context(&settings.fzf, &mut installed)? {
            Some(name) => name,
            None => return Ok(()),
        },
    };

    let kubeconfigs = kubeconfigs_matching(settings, &installed, &pattern, namespace_name.as_deref())?;
    print_paths(&kubeconfigs)
}

/// A kubeconfig per context matching `pattern`. Without an explicit namespace, each one
/// gets the namespace it was last used with.
fn kubeconfigs_matching(
    settings: &Settings,
    installed: &Installed,
    pattern: &str,
    namespace: Option<&str>,
) -> Result<Vec<KubeConfig>> {
    let matching = installed.get_contexts_matching(pattern, settings.behavior.allow_multiple_context_patterns);
    if matching.is_empty() {
        bail!("No context matching {pattern}");
    }

    let state = State::load()?;
    matching
        .into_iter()
        .map(|c| {
            let name = &c.item.name;
            let ns = namespace.or_else(|| state.namespace_history.get(name).and_then(|s| s.as_deref()));
            installed.make_kubeconfig_for_context(name, ns)
        })
        .collect()
}

fn print_paths(kubeconfigs: &[KubeConfig]) -> Result<()> {
    for kubeconfig in kubeconfigs {
        // Kept only once written, so a failed write leaves no file behind.
        let file = tempfile::Builder::new()
            .prefix("kubie-config")
            .suffix(".yaml")
            .tempfile()?;
        kubeconfig.write_to_file(file.path())?;
        let (_, path) = file.keep()?;
        println!("{}", path.display());
    }
    Ok(())
}
