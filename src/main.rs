use anyhow::Result;
use clap::Parser;

use cmd::meta::Kubie;
use settings::Settings;
use skim::prelude::SkimOptionsBuilder;

mod cmd;
mod ioutil;
mod kubeconfig;
mod kubectl;
mod session;
mod settings;
mod shell;
mod state;
mod vars;

fn main() -> Result<()> {
    let mut settings = Settings::load()?;

    let skim_options = {
        let mut options = SkimOptionsBuilder::default();

        options.no_multi(true);

        options.color(settings.fzf.color.take());

        if settings.fzf.ignore_case {
            options.case(skim::CaseMatching::Ignore);
        };

        if !settings.fzf.mouse {
            options.no_mouse(true);
        };

        if settings.fzf.reverse {
            options.reverse(true);
        }

        if settings.fzf.info_hidden {
            options.no_info(true);
        }

        if let Some(prompt) = settings.fzf.prompt.take() {
            options.prompt(prompt);
        }

        options.build().unwrap()
    };

    let kubie = Kubie::parse();

    match kubie {
        Kubie::Context {
            namespace_name,
            context_name,
            kubeconfigs,
            recursive,
            last_used,
        } => {
            cmd::context::context(
                &settings,
                &skim_options,
                context_name,
                namespace_name,
                kubeconfigs,
                recursive,
                last_used,
            )?;
        }
        Kubie::Namespace {
            namespace_name,
            recursive,
            unset,
        } => {
            cmd::namespace::namespace(&settings, &skim_options, namespace_name, recursive, unset)?;
        }
        Kubie::Info(info) => {
            cmd::info::info(info)?;
        }
        Kubie::Exec {
            context_name,
            namespace_name,
            exit_early,
            context_headers_flag,
            args,
        } => {
            cmd::exec::exec(
                &settings,
                context_name,
                namespace_name,
                exit_early,
                context_headers_flag,
                args,
            )?;
        }
        Kubie::Lint => {
            cmd::lint::lint(&settings)?;
        }
        Kubie::Edit { context_name } => {
            cmd::edit::edit_context(&settings, &skim_options, context_name)?;
        }
        Kubie::EditConfig => {
            cmd::edit::edit_config(&settings)?;
        }
        #[cfg(feature = "update")]
        Kubie::Update => {
            cmd::update::update()?;
        }
        Kubie::Delete { context_name } => {
            cmd::delete::delete_context(&settings, &skim_options, context_name)?;
        }
        Kubie::Export {
            context_name,
            namespace_name,
        } => {
            cmd::export::export(&settings, context_name, namespace_name)?;
        }
        Kubie::GenerateCompletion(cmd) => {
            cmd::meta::generate_completion(cmd);
        }
    }

    Ok(())
}
