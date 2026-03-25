use std::env;
use std::process::Command;
use std::str;

use anyhow::{anyhow, Context};

use crate::kubeconfig::KubeConfig;

pub fn get_namespaces<'a>(kubeconfig: impl Into<Option<&'a KubeConfig>>) -> anyhow::Result<Vec<String>> {
    let mut cmd = Command::new("kubectl");
    cmd.arg("get");
    cmd.arg("namespaces");

    let temp_config_file;

    if let Some(kubeconfig) = kubeconfig.into() {
        temp_config_file = tempfile::Builder::new()
            .prefix("kubie-config")
            .suffix(".yaml")
            .tempfile()?;
        kubeconfig.write_to_file(temp_config_file.path())?;
        cmd.env("KUBECONFIG", temp_config_file.path());
    } else {
        cmd.env(
            "KUBECONFIG",
            env::var("KUBIE_KUBECONFIG").context("KUBIE_KUBECONFIG variable is not set")?,
        );
    }

    let result = cmd.output()?;
    if !result.status.success() {
        let stderr = str::from_utf8(&result.stderr).unwrap_or("could not decode stderr of kubectl as utf-8");
        return Err(anyhow!("Error calling kubectl:\n{}", stderr));
    }

    let text = str::from_utf8(&result.stdout)?;
    let mut namespaces = vec![];
    for line in text.lines().skip(1) {
        let idx = line.find(' ').unwrap_or(line.len());
        namespaces.push(line[..idx].to_string());
    }

    Ok(namespaces)
}
