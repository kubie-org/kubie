use anyhow::Result;

use crate::kubeconfig::{self, KubeConfig};
use crate::session::Session;
use crate::settings::Settings;
use crate::shell::spawn_shell;

use super::eval;

#[derive(Debug)]
pub enum ActivationMode {
    Eval,
    Spawn,
    Switch,
}

impl ActivationMode {
    pub fn resolve(eval: bool, recursive: bool, is_active: bool) -> Self {
        if eval {
            ActivationMode::Eval
        } else if is_active && !recursive {
            ActivationMode::Switch
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