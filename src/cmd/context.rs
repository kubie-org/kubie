use std::env;
use std::fmt::Display;
use std::fs;

use anyhow::{Context, Result};
use skim::SkimOptions;

use crate::cmd::{select_or_list_context, SelectResult};
use crate::kubeconfig::{self, Installed, KubeConfig};
use crate::kubectl;
use crate::session::Session;
use crate::settings::Settings;
use crate::shell::spawn_shell;
use crate::state::State;
use crate::vars;

/// Prepares kubeconfig and session for context switching
fn prepare_context_switch(
    settings: &Settings,
    installed: &Installed,
    context_name: &str,
    namespace_name: Option<&str>,
) -> Result<(KubeConfig, Session)> {
    let state = State::load()?;
    let mut session = Session::load()?;

    let namespace_name =
        namespace_name.or_else(|| state.namespace_history.get(context_name).and_then(|s| s.as_deref()));

    let kubeconfig = if context_name == "-" {
        let previous_ctx = session
            .get_last_context()
            .context("There is no previous context to switch to.")?;
        installed.make_kubeconfig_for_context(&previous_ctx.context, previous_ctx.namespace.as_deref())?
    } else {
        installed.make_kubeconfig_for_context(context_name, namespace_name)?
    };

    session.add_history_entry(
        &kubeconfig.contexts[0].name,
        kubeconfig.contexts[0].context.namespace.as_deref(),
    );

    if settings.behavior.validate_namespaces.can_list_namespaces() {
        if let Some(namespace_name) = namespace_name {
            let namespaces = kubectl::get_namespaces(Some(&kubeconfig))?;
            if !namespaces.iter().any(|x| x == namespace_name) {
                eprintln!("Warning: namespace {namespace_name} does not exist.");
            }
        }
    }

    Ok((kubeconfig, session))
}

fn cleanup_previous_session() {
    if let Some(prev_kubeconfig) = env::var_os("KUBIE_KUBECONFIG") {
        let _ = fs::remove_file(prev_kubeconfig);
    }
    if let Some(prev_session) = env::var_os("KUBIE_SESSION") {
        let _ = fs::remove_file(prev_session);
    }
}

/// Prints a shell export statement with proper escaping for special characters
fn export_var(key: &str, value: impl Display) {
    let value_str = value.to_string();
    let quoted = shlex::try_quote(&value_str).unwrap_or(std::borrow::Cow::Borrowed(&value_str));
    println!("export {}={}", key, quoted);
}

fn enter_context(
    settings: &Settings,
    installed: Installed,
    context_name: &str,
    namespace_name: Option<&str>,
    recursive: bool,
) -> Result<()> {
    let (kubeconfig, session) = prepare_context_switch(settings, &installed, context_name, namespace_name)?;

    if vars::is_kubie_active() && !recursive {
        let path = kubeconfig::get_kubeconfig_path()?;
        kubeconfig.write_to_file(path.as_path())?;
        session.save(None)?;
    } else {
        spawn_shell(settings, kubeconfig, &session)?;
    }

    Ok(())
}

fn enter_context_for_eval(
    settings: &Settings,
    installed: Installed,
    context_name: &str,
    namespace_name: Option<&str>,
) -> Result<()> {
    let (kubeconfig, session) = prepare_context_switch(settings, &installed, context_name, namespace_name)?;

    cleanup_previous_session();

    let temp_config_file = tempfile::Builder::new()
        .prefix("kubie-config-")
        .suffix(".yaml")
        .tempfile()?;
    kubeconfig.write_to_file(temp_config_file.path())?;

    let temp_session_file = tempfile::Builder::new()
        .prefix("kubie-session-")
        .suffix(".json")
        .tempfile()?;
    session.save(Some(temp_session_file.path()))?;

    let next_depth = vars::get_depth() + 1;

    export_var("KUBECONFIG", temp_config_file.path().display());
    export_var("KUBIE_ACTIVE", "1");
    export_var("KUBIE_DEPTH", next_depth);
    export_var("KUBIE_KUBECONFIG", temp_config_file.path().display());
    export_var("KUBIE_SESSION", temp_session_file.path().display());

    // Will be cleaned up on next switch or shell exit
    let _ = temp_config_file.into_temp_path().keep();
    let _ = temp_session_file.into_temp_path().keep();

    Ok(())
}

pub fn context(
    settings: &Settings,
    skim_options: &SkimOptions,
    context_name: Option<String>,
    namespace_name: Option<String>,
    kubeconfigs: Vec<String>,
    recursive: bool,
    eval: bool,
) -> Result<()> {
    let mut installed = if kubeconfigs.is_empty() {
        kubeconfig::get_installed_contexts(settings)?
    } else {
        kubeconfig::get_kubeconfigs_contexts(&kubeconfigs)?
    };

    let context_name = match context_name {
        Some(context_name) => context_name,
        None => match select_or_list_context(skim_options, &mut installed)? {
            SelectResult::Selected(x) => x,
            _ => return Ok(()),
        },
    };

    if eval {
        return enter_context_for_eval(settings, installed, &context_name, namespace_name.as_deref());
    }

    enter_context(settings, installed, &context_name, namespace_name.as_deref(), recursive)
}
