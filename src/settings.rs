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

impl Settings {
    pub fn path() -> String {
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

        // Very important to exclude kubie's own config file ~/.kube/kubie.yaml from the results.
        settings.configs.exclude.push(settings_path_str);
        Ok(settings)
    }

    pub fn get_kube_configs_paths(&self) -> Result<HashSet<PathBuf>> {
        let mut paths = HashSet::new();
        for inc in &self.configs.include {
            let expanded = expanduser(inc);
            for entry in glob(&expanded)? {
                paths.insert(entry?);
            }
        }

        for exc in &self.configs.exclude {
            let expanded = expanduser(exc);
            for entry in glob(&expanded)? {
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
    #[serde(default = "def_bool_false")]
    pub track_last_used: bool,
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

#[test]
fn test_expanduser() {
    assert_eq!(
        expanduser("~/hello/world/*.foo"),
        format!("{}/hello/world/*.foo", home_dir())
    );
}
