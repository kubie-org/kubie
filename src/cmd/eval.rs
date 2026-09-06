use std::env;
use std::fmt::Display;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::kubeconfig::KubeConfig;
use crate::session::Session;
use crate::settings::Settings;
use crate::shell::{detect_shell, ShellKind};
use crate::vars;

// Distinct from spawn_shell's own prefix so is_managed_file never confuses the two.
const CONFIG_PREFIX: &str = "kubie-eval-config-";
const SESSION_PREFIX: &str = "kubie-eval-session-";

pub(super) fn emit_session(settings: &Settings, config: &KubeConfig, session: &Session) -> Result<()> {
    let shell = detect_eval_shell(settings)?;

    let (config_path, session_path) = match existing_eval_paths() {
        // Reuse an existing --eval pair in place rather than minting a new one.
        Some((config_path, session_path)) => {
            config.write_to_file(&config_path)?;
            session.save(Some(&session_path))?;
            (config_path, session_path)
        }
        None => create_eval_files(config, session)?,
    };

    let depth = if vars::is_kubie_active() { vars::get_depth() } else { 1 };
    let config_path = config_path.display().to_string();
    let session_path = session_path.display().to_string();

    emit_vars(shell, &config_path, &session_path, depth);

    Ok(())
}

fn create_eval_files(config: &KubeConfig, session: &Session) -> Result<(PathBuf, PathBuf)> {
    let temp_config_file = tempfile::Builder::new()
        .prefix(CONFIG_PREFIX)
        .suffix(".yaml")
        .tempfile()?;
    config.write_to_file(temp_config_file.path())?;

    let temp_session_file = tempfile::Builder::new()
        .prefix(SESSION_PREFIX)
        .suffix(".json")
        .tempfile()?;
    session.save(Some(temp_session_file.path()))?;

    let config_path = temp_config_file.into_temp_path().keep()?;
    let session_path = temp_session_file.into_temp_path().keep()?;

    Ok((config_path, session_path))
}

/// Paths of the kubeconfig/session pair from a previous `--eval` call, if still active.
fn existing_eval_paths() -> Option<(PathBuf, PathBuf)> {
    if !vars::is_kubie_active() {
        return None;
    }

    let config_path = PathBuf::from(env::var_os("KUBIE_KUBECONFIG")?);
    let session_path = PathBuf::from(env::var_os("KUBIE_SESSION")?);

    if is_managed_file(&config_path, CONFIG_PREFIX) && is_managed_file(&session_path, SESSION_PREFIX) {
        Some((config_path, session_path))
    } else {
        None
    }
}

fn is_managed_file(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(prefix))
}

fn detect_eval_shell(settings: &Settings) -> Result<ShellKind> {
    let shell = match &settings.shell {
        Some(s) => ShellKind::from_str(s).ok_or_else(|| anyhow!("Invalid shell setting: {}", s))?,
        None => detect_shell()?,
    };

    match shell {
        ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish => Ok(shell),
        _ => Err(anyhow!(
            "--eval is not supported for this shell. Supported: bash, zsh, fish."
        )),
    }
}

fn emit_vars(shell: ShellKind, config_path: &str, session_path: &str, depth: impl Display) {
    println!("{}", render_vars(shell, config_path, session_path, depth).join("\n"));
}

fn render_vars(shell: ShellKind, config_path: &str, session_path: &str, depth: impl Display) -> Vec<String> {
    vec![
        format_var(shell, "KUBECONFIG", config_path),
        format_var(shell, "KUBIE_ACTIVE", "1"),
        format_var(shell, "KUBIE_DEPTH", depth),
        format_var(shell, "KUBIE_KUBECONFIG", config_path),
        format_var(shell, "KUBIE_SESSION", session_path),
    ]
}

fn format_var(shell: ShellKind, key: &str, value: impl Display) -> String {
    let value_str = value.to_string();
    let quoted = shlex::try_quote(&value_str).unwrap_or(std::borrow::Cow::Borrowed(&value_str));
    match shell {
        ShellKind::Bash | ShellKind::Zsh => format!("export {}={};", key, quoted),
        ShellKind::Fish => format!("set -gx {} {};", key, quoted),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{is_managed_file, render_vars, ShellKind, CONFIG_PREFIX};

    #[test]
    fn test_is_managed_file_accepts_matching_prefix() {
        let path = Path::new("/tmp").join(format!("{CONFIG_PREFIX}abc123.yaml"));
        assert!(is_managed_file(&path, CONFIG_PREFIX));
    }

    #[test]
    fn test_is_managed_file_rejects_other_prefix() {
        let path = Path::new("/tmp").join("kubie-config123.yaml");
        assert!(!is_managed_file(&path, CONFIG_PREFIX));
    }

    #[test]
    fn test_bash_output_uses_export_syntax() {
        let script = render_vars(ShellKind::Bash, "/tmp/config.yaml", "/tmp/session.json", 1).join("\n");

        assert!(script.contains("export KUBECONFIG=/tmp/config.yaml;"));
        assert!(script.contains("export KUBIE_ACTIVE=1;"));
        assert!(script.contains("export KUBIE_DEPTH=1;"));
        assert!(script.contains("export KUBIE_SESSION=/tmp/session.json;"));
    }

    #[test]
    fn test_fish_output_uses_set_gx_syntax() {
        let script = render_vars(ShellKind::Fish, "/tmp/config.yaml", "/tmp/session.json", 2).join("\n");

        assert!(script.contains("set -gx KUBECONFIG /tmp/config.yaml;"));
        assert!(script.contains("set -gx KUBIE_DEPTH 2;"));
        assert!(script.contains("set -gx KUBIE_SESSION /tmp/session.json;"));
    }
}
