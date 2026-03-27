use anyhow::{anyhow, Context, Result};

use crate::cmd::{select_or_list_namespace, SelectResult};
use crate::kubeconfig;
use crate::kubectl;
use crate::session::Session;
use crate::settings::{Settings, ValidateNamespacesBehavior};
use crate::shell::spawn_shell;
use crate::state::State;
use crate::vars;

pub fn namespace(
    settings: &Settings,
    namespace_name: Option<String>,
    recursive: bool,
    unset: bool,
) -> Result<()> {
    vars::ensure_kubie_active()?;

    let mut session = Session::load().context("Could not load session file")?;

    if namespace_name.is_none() && unset {
        return enter_namespace(settings, &mut session, recursive, None);
    }

    let namespace_name = match namespace_name {
        Some(s) if s == "-" => Some(
            session
                .get_last_namespace()
                .context("There is not previous namespace to switch to")?
                .to_string(),
        ),
        Some(s) => match settings.behavior.validate_namespaces {
            ValidateNamespacesBehavior::False => Some(s),
            ValidateNamespacesBehavior::True => {
                let namespaces = kubectl::get_namespaces(None)?;
                if !namespaces.contains(&s) {
                    return Err(anyhow!("'{}' is not a valid namespace for the context", s));
                }
                Some(s)
            }
            ValidateNamespacesBehavior::Partial => {
                let namespaces = kubectl::get_namespaces(None)?;
                if namespaces.contains(&s) {
                    Some(s)
                } else {
                    let ns_partial_matches: Vec<String> =
                        namespaces.iter().filter(|&ns| ns.contains(&s)).cloned().collect();
                    match ns_partial_matches.len() {
                        0 => return Err(anyhow!("'{}' is not a valid namespace for the context", s)),
                        1 => Some(ns_partial_matches[0].clone()),
                        _ => match select_or_list_namespace(&settings.fzf, Some(ns_partial_matches))? {
                            SelectResult::Selected(s) => Some(s),
                            _ => return Ok(()),
                        },
                    }
                }
            }
        },
        None => match select_or_list_namespace(&settings.fzf, None)? {
            SelectResult::Selected(s) => Some(s),
            _ => return Ok(()),
        },
    };

    enter_namespace(settings, &mut session, recursive, namespace_name)
}

fn enter_namespace(
    settings: &Settings,
    session: &mut Session,
    recursive: bool,
    namespace_name: Option<String>,
) -> Result<()> {
    let mut config = kubeconfig::get_current_config()?;
    config.contexts[0].context.namespace = namespace_name.clone();

    let context_name = &config.contexts[0].name;

    // Update the state, set the last namespace used for the context.
    // We take out a file lock here to avoid concurrent kubie processes
    // corrupting the state file
    State::modify(|state| {
        state
            .namespace_history
            .insert(context_name.into(), namespace_name.clone());
        Ok(())
    })?;

    // Update the history, add the context and namespace to it.
    session.add_history_entry(context_name, namespace_name);

    if recursive {
        spawn_shell(settings, config, session)?;
    } else {
        let config_file = kubeconfig::get_kubeconfig_path()?;
        config.write_to_file(config_file.as_path())?;
        session.save(None)?;
    }

    Ok(())
}
