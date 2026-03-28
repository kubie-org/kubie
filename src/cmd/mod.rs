use std::io::{self, IsTerminal};

use anyhow::{bail, Context, Result};

use crate::kubeconfig::{self, Installed, KubeConfig};
use crate::kubectl;
use crate::session::Session;
use crate::settings::{Fzf, Settings};
use crate::shell::spawn_shell;
use crate::vars;

pub mod context;
pub mod delete;
mod eval;
pub mod edit;
pub mod exec;
pub mod export;
pub mod info;
pub mod lint;
pub mod meta;
pub mod namespace;
#[cfg(feature = "update")]
pub mod update;

#[derive(Debug, PartialEq)]
pub enum ActivationMode {
    Eval,
    Spawn,
    Switch,
}

impl ActivationMode {
    pub fn resolve(eval: bool, recursive: bool) -> Self {
        if vars::is_kubie_active() && !recursive {
            ActivationMode::Switch
        } else if eval {
            ActivationMode::Eval
        } else {
            ActivationMode::Spawn
        }
    }

    pub fn activate(self, settings: &Settings, config: KubeConfig, session: &Session) -> Result<()> {
        match self {
            ActivationMode::Eval => eval::emit_session(settings, &config, session),
            ActivationMode::Spawn => {
                spawn_shell(settings, config, session)?;
                Ok(())
            }
            ActivationMode::Switch => {
                let path = kubeconfig::get_kubeconfig_path()?;
                config.write_to_file(path.as_path())?;
                session.save(None)?;
                Ok(())
            }
        }
    }
}

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

    if io::stdin().is_terminal() {
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

    if io::stdin().is_terminal() {
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

#[cfg(test)]
pub(crate) mod test_support {
    use std::env;
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};

    pub(crate) fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    pub(crate) struct EnvVarGuard {
        key: &'static str,
        prev: Option<OsString>,
    }

    impl EnvVarGuard {
        pub(crate) fn set(key: &'static str, value: &'static str) -> Self {
            let prev = env::var_os(key);
            env::set_var(key, value);
            Self { key, prev }
        }

        pub(crate) fn unset(key: &'static str) -> Self {
            let prev = env::var_os(key);
            env::remove_var(key);
            Self { key, prev }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(value) => env::set_var(self.key, value),
                None => env::remove_var(self.key),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ActivationMode;
    use super::test_support::{env_lock, EnvVarGuard};

    fn resolve_with_active(active: bool, eval: bool, recursive: bool) -> ActivationMode {
        let _env_lock = env_lock().lock().unwrap();
        let _guard = if active {
            EnvVarGuard::set("KUBIE_ACTIVE", "1")
        } else {
            EnvVarGuard::unset("KUBIE_ACTIVE")
        };

        ActivationMode::resolve(eval, recursive)
    }

    #[test]
    fn test_resolve_spawns_shell_by_default() {
        assert_eq!(resolve_with_active(false, false, false), ActivationMode::Spawn);
    }

    #[test]
    fn test_resolve_returns_eval_when_flag_set() {
        assert_eq!(resolve_with_active(false, true, false), ActivationMode::Eval);
    }

    #[test]
    fn test_resolve_switches_in_active_session() {
        assert_eq!(resolve_with_active(true, false, false), ActivationMode::Switch);
    }

    #[test]
    fn test_resolve_active_session_overrides_eval_flag() {
        assert_eq!(resolve_with_active(true, true, false), ActivationMode::Switch);
    }

    #[test]
    fn test_resolve_recursive_bypasses_active_session() {
        assert_eq!(resolve_with_active(true, false, true), ActivationMode::Spawn);
    }
}
