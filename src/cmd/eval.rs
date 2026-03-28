use std::env;
use std::fmt::Display;
use std::fs;

use anyhow::{anyhow, Result};

use crate::kubeconfig::KubeConfig;
use crate::session::Session;
use crate::settings::Settings;
use crate::shell::{detect_shell, ShellKind};
use crate::vars;

pub(super) fn emit_session(settings: &Settings, config: &KubeConfig, session: &Session) -> Result<()> {
    let shell = detect_eval_shell(settings)?;

    cleanup_previous_session();

    let temp_config_file = tempfile::Builder::new()
        .prefix("kubie-config-")
        .suffix(".yaml")
        .tempfile()?;
    config.write_to_file(temp_config_file.path())?;

    let temp_session_file = tempfile::Builder::new()
        .prefix("kubie-session-")
        .suffix(".json")
        .tempfile()?;
    session.save(Some(temp_session_file.path()))?;

    let depth = if vars::is_kubie_active() { vars::get_depth() } else { 1 };
    let config_path = temp_config_file.path().display().to_string();
    let session_path = temp_session_file.path().display().to_string();

    emit_vars(shell, &config_path, &session_path, depth);

    let _ = temp_config_file.into_temp_path().keep();
    let _ = temp_session_file.into_temp_path().keep();

    Ok(())
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

fn cleanup_previous_session() {
    if let Some(prev_kubeconfig) = env::var_os("KUBIE_KUBECONFIG") {
        let _ = fs::remove_file(prev_kubeconfig);
    }
    if let Some(prev_session) = env::var_os("KUBIE_SESSION") {
        let _ = fs::remove_file(prev_session);
    }
}

fn emit_vars(shell: ShellKind, config_path: &str, session_path: &str, depth: impl Display) {
    for line in render_vars(shell, config_path, session_path, depth) {
        println!("{line}");
    }
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
    use super::{render_vars, ShellKind};

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
