use anyhow::{bail, Result};

use crate::cmd::prompt_context;
use crate::kubeconfig::{self, Installed, KubeConfig};
use crate::session::Session;
use crate::settings::Settings;
use crate::shell;
use crate::state::State;
use crate::vars;

/// Print a kubeconfig path per matched context, or with `as_session` the environment that
/// lets the calling shell enter the context. Nothing else goes to stdout.
pub fn export(
    settings: &Settings,
    context_name: Option<String>,
    namespace_name: Option<String>,
    as_session: bool,
) -> Result<()> {
    let mut installed = kubeconfig::get_installed_contexts(settings)?;

    let pattern = match context_name {
        Some(name) => name,
        None => match prompt_context(&settings.fzf, &mut installed)? {
            Some(name) => name,
            None => return Ok(()),
        },
    };

    let kubeconfigs = kubeconfigs_matching(settings, &installed, &pattern, namespace_name.as_deref())?;
    if as_session {
        print_session(&kubeconfigs)
    } else {
        print_paths(&kubeconfigs)
    }
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

/// Write the files a kubie shell runs with and print the variables that point to them.
/// Nothing removes the files when the shell exits.
fn print_session(kubeconfigs: &[KubeConfig]) -> Result<()> {
    // A session holds one shell's history, so it can only follow one context.
    let [kubeconfig] = kubeconfigs else {
        bail!("--session needs a single context, but {} matched", kubeconfigs.len());
    };

    // Start from the current history, so `-` still leads back.
    let mut session = Session::load()?;
    session.record_context_entry(
        &kubeconfig.contexts[0].name,
        kubeconfig.contexts[0].context.namespace.as_deref(),
    )?;

    let (config, session_file) = shell::write_shell_files(kubeconfig, &session)?;
    let (_, config) = config.keep()?;
    let (_, session_file) = session_file.keep()?;

    // Activating twice in a row is still one shell deep.
    let depth = vars::get_depth().max(1);

    // Raw values, one per line. Callers split on newlines, so quoting would end up in the value.
    println!("KUBECONFIG={}", config.display());
    println!("KUBIE_KUBECONFIG={}", config.display());
    println!("KUBIE_SESSION={}", session_file.display());
    println!("KUBIE_ACTIVE=1");
    println!("KUBIE_DEPTH={depth}");
    Ok(())
}
