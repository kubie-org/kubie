use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, IsTerminal};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glob::glob;
use lazy_static::lazy_static;
use serde::Deserialize;

lazy_static! {
    static ref HOME_DIR: String = dirs::home_dir()
        .expect("could not get home directory path")
        .to_str()
        .expect("home directory contains non unicode characters")
        .to_string();
}

#[inline]
fn home_dir() -> &'static str {
    &HOME_DIR
}

pub fn expanduser(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        format!("{}/{}", home_dir(), stripped)
    } else {
        path.to_string()
    }
}

#[derive(Default, Debug, Deserialize)]
pub struct Fzf {
    pub mouse: bool,
    pub reverse: bool,
    pub ignore_case: bool,
    pub info_hidden: bool,
    pub prompt: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub default_editor: Option<String>,
    #[serde(default)]
    pub configs: Configs,
    #[serde(default)]
    pub prompt: Prompt,
    #[serde(default)]
    pub behavior: Behavior,
    #[serde(default)]
    pub hooks: Hooks,
    #[serde(default)]
    pub fzf: Fzf,
}

/// Check if a path has a kubie settings filename.
fn is_kubie_settings_name(path: &Path) -> bool {
    matches!(path.file_name().and_then(|f| f.to_str()), Some("kubie.yaml") | Some("kubie.yml"))
}

/// Parse KUBECONFIG into individual entries (colon-separated on unix).
fn parse_kubeconfig_env() -> Vec<PathBuf> {
    match std::env::var("KUBECONFIG") {
        Ok(val) if !val.is_empty() => val.split(':').map(PathBuf::from).collect(),
        _ => vec![],
    }
}

/// Find a kubie settings file (kubie.yaml or kubie.yml) in a directory.
fn find_settings_in_dir(dir: &Path) -> Option<String> {
    ["kubie.yaml", "kubie.yml"].iter().find_map(|name| {
        let c = dir.join(name);
        if c.is_file() { c.to_str().map(String::from) } else { None }
    })
}

impl Settings {
    pub fn path() -> String {
        for entry in parse_kubeconfig_env() {
            if is_kubie_settings_name(&entry) && entry.is_file() {
                if let Some(s) = entry.to_str() {
                    return s.to_string();
                }
            }
            if let Some(s) = find_settings_in_dir(&entry) {
                return s;
            }
        }

        let xdg_config = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{}/.config", home_dir()));
        let xdg_dir = Path::new(&xdg_config).join("kubie");
        if let Some(s) = find_settings_in_dir(&xdg_dir) {
            return s;
        }

        format!("{}/.kube/kubie.yaml", home_dir())
    }

    pub fn load() -> Result<Settings> {
        let settings_path_str = Self::path();
        let settings_path = Path::new(&settings_path_str);

        let mut settings = if settings_path.exists() {
            let file = File::open(settings_path)?;
            let reader = BufReader::new(file);
            serde_yaml::from_reader(reader).context("could not parse kubie config")?
        } else {
            Settings::default()
        };

        // Very important to exclude kubie's own config file from the results.
        settings.configs.exclude.push(settings_path_str);
        Ok(settings)
    }

    pub fn get_kube_configs_paths(&self) -> Result<HashSet<PathBuf>> {
        let mut paths = HashSet::new();

        for entry in parse_kubeconfig_env() {
            if entry.is_file() && !is_kubie_settings_name(&entry) {
                paths.insert(entry);
            } else if entry.is_dir() {
                for pattern in &["*.yml", "*.yaml"] {
                    for matched in glob(&format!("{}/{pattern}", entry.display()))? {
                        let path = matched?;
                        if !is_kubie_settings_name(&path) {
                            paths.insert(path);
                        }
                    }
                }
            }
        }

        for inc in &self.configs.include {
            for entry in glob(&expanduser(inc))? {
                paths.insert(entry?);
            }
        }

        for exc in &self.configs.exclude {
            for entry in glob(&expanduser(exc))? {
                paths.remove(&entry?);
            }
        }

        Ok(paths)
    }
}

#[derive(Debug, Deserialize)]
pub struct Configs {
    #[serde(default = "default_include_path")]
    pub include: Vec<String>,
    #[serde(default = "default_exclude_path")]
    pub exclude: Vec<String>,
}

impl Default for Configs {
    fn default() -> Self {
        Configs {
            include: default_include_path(),
            exclude: default_exclude_path(),
        }
    }
}

fn default_include_path() -> Vec<String> {
    let home_dir = home_dir();
    vec![
        format!("{home_dir}/.kube/config"),
        format!("{home_dir}/.kube/*.yml"),
        format!("{home_dir}/.kube/*.yaml"),
        format!("{home_dir}/.kube/configs/*.yml"),
        format!("{home_dir}/.kube/configs/*.yaml"),
        format!("{home_dir}/.kube/kubie/*.yml"),
        format!("{home_dir}/.kube/kubie/*.yaml"),
    ]
}

fn default_exclude_path() -> Vec<String> {
    vec![]
}

#[derive(Debug, Deserialize)]
pub struct Prompt {
    #[serde(default = "def_bool_false")]
    pub disable: bool,
    #[serde(default = "def_bool_true")]
    pub show_depth: bool,
    #[serde(default = "def_bool_false")]
    pub zsh_use_rps1: bool,
    #[serde(default = "def_bool_false")]
    pub fish_use_rprompt: bool,
    #[serde(default = "def_bool_false")]
    pub xonsh_use_right_prompt: bool,
}

impl Default for Prompt {
    fn default() -> Self {
        Prompt {
            disable: false,
            show_depth: true,
            zsh_use_rps1: false,
            fish_use_rprompt: false,
            xonsh_use_right_prompt: false,
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum, Deserialize)]
#[clap(rename_all = "lower")]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ContextHeaderBehavior {
    #[default]
    Auto,
    Always,
    Never,
}

impl ContextHeaderBehavior {
    pub fn should_print_headers(&self) -> bool {
        match self {
            ContextHeaderBehavior::Auto => io::stdout().is_terminal(),
            ContextHeaderBehavior::Always => true,
            ContextHeaderBehavior::Never => false,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct Behavior {
    #[serde(default)]
    pub validate_namespaces: ValidateNamespacesBehavior,
    #[serde(default)]
    pub print_context_in_exec: ContextHeaderBehavior,
    #[serde(default = "def_bool_false")]
    pub allow_multiple_context_patterns: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ValidateNamespacesBehavior {
    #[default]
    True,
    False,
    Partial,
}

impl ValidateNamespacesBehavior {
    pub fn can_list_namespaces(&self) -> bool {
        match self {
            ValidateNamespacesBehavior::True | ValidateNamespacesBehavior::Partial => true,
            ValidateNamespacesBehavior::False => false,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct Hooks {
    #[serde(default)]
    pub start_ctx: String,
    #[serde(default)]
    pub stop_ctx: String,
}

fn def_bool_true() -> bool {
    true
}

fn def_bool_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_expanduser() {
        assert_eq!(
            expanduser("~/hello/world/*.foo"),
            format!("{}/hello/world/*.foo", home_dir())
        );
    }

    #[test]
    fn test_kubeconfig_env_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test-cluster.yaml");
        fs::write(&file_path, "apiVersion: v1").unwrap();

        std::env::set_var("KUBECONFIG", file_path.to_str().unwrap());

        let settings = Settings {
            configs: Configs {
                include: vec![],
                exclude: vec![],
            },
            ..Settings::default()
        };

        let paths = settings.get_kube_configs_paths().unwrap();
        assert!(paths.contains(&file_path));

        std::env::remove_var("KUBECONFIG");
    }

    #[test]
    fn test_kubeconfig_env_directory() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.yaml"), "apiVersion: v1").unwrap();
        fs::write(dir.path().join("b.yml"), "apiVersion: v1").unwrap();
        fs::write(dir.path().join("c.txt"), "not a kubeconfig").unwrap();
        fs::write(dir.path().join("kubie.yaml"), "configs: {}").unwrap();

        std::env::set_var("KUBECONFIG", dir.path().to_str().unwrap());

        let settings = Settings {
            configs: Configs {
                include: vec![],
                exclude: vec![],
            },
            ..Settings::default()
        };

        let paths = settings.get_kube_configs_paths().unwrap();
        assert!(paths.contains(&dir.path().join("a.yaml")));
        assert!(paths.contains(&dir.path().join("b.yml")));
        assert!(!paths.contains(&dir.path().join("c.txt")));
        assert!(!paths.contains(&dir.path().join("kubie.yaml")));

        std::env::remove_var("KUBECONFIG");
    }

    #[test]
    fn test_kubeconfig_env_picks_up_kubie_settings() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("kubie.yaml"), "configs: {}").unwrap();

        std::env::set_var("KUBECONFIG", dir.path().to_str().unwrap());

        let path = Settings::path();
        assert_eq!(path, dir.path().join("kubie.yaml").to_str().unwrap());

        std::env::remove_var("KUBECONFIG");
    }

    #[test]
    fn test_kubeconfig_env_unset() {
        std::env::remove_var("KUBECONFIG");

        let settings = Settings {
            configs: Configs {
                include: vec![],
                exclude: vec![],
            },
            ..Settings::default()
        };

        let paths = settings.get_kube_configs_paths().unwrap();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_xdg_config_home() {
        std::env::remove_var("KUBECONFIG");
        let dir = tempfile::tempdir().unwrap();
        let kubie_dir = dir.path().join("kubie");
        fs::create_dir_all(&kubie_dir).unwrap();
        fs::write(kubie_dir.join("kubie.yaml"), "configs: {}").unwrap();

        std::env::set_var("XDG_CONFIG_HOME", dir.path().to_str().unwrap());

        let path = Settings::path();
        assert_eq!(path, kubie_dir.join("kubie.yaml").to_str().unwrap());

        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
