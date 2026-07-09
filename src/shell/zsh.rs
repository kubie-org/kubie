use std::fs::File;
use std::io::{BufWriter, Write};
use std::process::Command;

use anyhow::{Context, Result};
use tempfile::tempdir;

use super::ShellSpawnInfo;

pub fn spawn_shell(info: &ShellSpawnInfo) -> Result<()> {
    let dir = tempdir()?;
    {
        let zshrc_path = dir.path().join(".zshrc");
        let zshrc = File::create(zshrc_path).context("Could not open zshrc file")?;
        let mut zshrc_buf = BufWriter::new(zshrc);
        write!(
            zshrc_buf,
            r#"
# Reference for loading behavior
# https://shreevatsa.wordpress.com/2008/03/30/zshbash-startup-files-loading-order-bashrc-zshrc-etc/

if [[ -f "/etc/zshenv" ]] ; then
    source "/etc/zshenv"
elif [[ -f "/etc/zsh/zshenv" ]] ; then
    source "/etc/zsh/zshenv"
fi

if [[ -f "$HOME/.zshenv" ]] ; then
    tmp_ZDOTDIR=$ZDOTDIR
    source "$HOME/.zshenv"
    # If the user has overridden $ZDOTDIR, we save that in $_KUBIE_USER_ZDOTDIR for later reference
    # and reset $ZDOTDIR
    if [[ "$tmp_ZDOTDIR" != "$ZDOTDIR" ]]; then
        _KUBIE_USER_ZDOTDIR=$ZDOTDIR
        ZDOTDIR=$tmp_ZDOTDIR
        unset tmp_ZDOTDIR
    fi
fi

# Configure HISTFILE to an existing .zsh_history unless already configured to preserve history.
if [[ -f "$HISTFILE" ]] ; then
    # We are using the user's configured history file.
    :
elif [[ -f "$_KUBIE_USER_ZDOTDIR/.zsh_history" ]] ; then
    export HISTFILE="$_KUBIE_USER_ZDOTDIR/.zsh_history"
elif [[ -f "$HOME/.zsh_history" ]] ; then
    export HISTFILE="$HOME/.zsh_history"
fi

KUBIE_LOGIN_SHELL=0
if [[ "$OSTYPE" == "darwin"* ]] ; then
    KUBIE_LOGIN_SHELL=1
fi

if [[ -f "/etc/zprofile" && "$KUBIE_LOGIN_SHELL" == "1" ]] ; then
    source "/etc/zprofile"
elif [[ -f "/etc/zsh/zprofile" && "$KUBIE_LOGIN_SHELL" == "1" ]] ; then
    source "/etc/zsh/zprofile"
fi

if [[ -f "${{_KUBIE_USER_ZDOTDIR:-$HOME}}/.zprofile" && "$KUBIE_LOGIN_SHELL" == "1" ]] ; then
    source "${{_KUBIE_USER_ZDOTDIR:-$HOME}}/.zprofile"
fi

if [[ -f "/etc/zshrc" ]] ; then
    source "/etc/zshrc"
elif [[ -f "/etc/zsh/zshrc" ]] ; then
    source "/etc/zsh/zshrc"
fi

if [[ -f "${{_KUBIE_USER_ZDOTDIR:-$HOME}}/.zshrc" ]] ; then
    ZDOTDIR=$_KUBIE_USER_ZDOTDIR \
        source "${{_KUBIE_USER_ZDOTDIR:-$HOME}}/.zshrc"
fi

if [[ -f "/etc/zlogin" && "$KUBIE_LOGIN_SHELL" == "1" ]] ; then
    source "/etc/zlogin"
elif [[ -f "/etc/zsh/zlogin" && "$KUBIE_LOGIN_SHELL" == "1" ]] ; then
    source "/etc/zsh/zlogin"
fi

if [[ -f "${{_KUBIE_USER_ZDOTDIR:-$HOME}}/.zlogin" && "$KUBIE_LOGIN_SHELL" == "1" ]] ; then
    source "${{_KUBIE_USER_ZDOTDIR:-$HOME}}/.zlogin"
fi

unset _KUBIE_USER_ZDOTDIR

autoload -Uz add-zsh-hook

# This function sets the proper KUBECONFIG variable before a command runs,
# in case something overwrote it.
function __kubie_cmd_pre_exec__() {{
    export KUBECONFIG="$KUBIE_KUBECONFIG"
}}

add-zsh-hook preexec __kubie_cmd_pre_exec__
"#,
        )?;

        if !info.settings.prompt.disable {
            write!(
                zshrc_buf,
                r#"
# Activate prompt substitution.
setopt PROMPT_SUBST

# This function fixes the prompt via a precmd hook.
function __kubie_cmd_pre_cmd__() {{
    local KUBIE_PROMPT=$'{}'

    # If KUBIE_ZSH_USE_RPS1 is set, we use RPS1 instead of PS1.
    if [[ "$KUBIE_ZSH_USE_RPS1" == "1" ]] ; then

        # Avoid modifying RPS1 again if the RPS1 has not been reset.
        if [[ "$RPS1" != *"$KUBIE_PROMPT"* ]] ; then

            # If RPS1 is empty, we do not seperate with a space.
            if [[ -z "$RPS1" ]] ; then
                RPS1="$KUBIE_PROMPT"
            else
                RPS1="$KUBIE_PROMPT $RPS1"
            fi
        fi
    else
        # Avoid modifying PS1 again if the PS1 has not been reset.
        if [[ "$PS1" != *"$KUBIE_PROMPT"* ]] ; then
            PS1="$KUBIE_PROMPT $PS1"
        fi
    fi
}}

# When promptinit is activated, a precmd hook which updates PS1 is installed.
# In order to inject the kubie PS1 when promptinit is activated, we must
# also add our own precmd hook which modifies PS1 after promptinit themes.
add-zsh-hook precmd __kubie_cmd_pre_cmd__
"#,
                info.prompt
            )?;
        }

        if !info.settings.hooks.start_ctx.is_empty() {
            write!(zshrc_buf, "{}", info.settings.hooks.start_ctx)?;
        }
    }

    let mut cmd = Command::new("zsh");
    cmd.env("ZDOTDIR", dir.path());
    info.env_vars.apply(&mut cmd);

    let mut child = cmd.spawn()?;
    child.wait()?;

    if !info.settings.hooks.stop_ctx.is_empty() {
        let temp_exit_hook_file = tempfile::Builder::new()
            .prefix("kubie-zsh-exit-hook")
            .suffix(".zsh")
            .tempfile()?;
        let mut temp_exit_hook_file_buf = BufWriter::new(temp_exit_hook_file.as_file());

        write!(temp_exit_hook_file_buf, "{}", info.settings.hooks.stop_ctx)?;

        temp_exit_hook_file_buf.flush()?;
        let mut exit_cmd = Command::new("zsh");
        exit_cmd.arg(temp_exit_hook_file.path());
        info.env_vars.apply(&mut exit_cmd);

        let mut child = exit_cmd.spawn()?;
        child.wait()?;
    }

    Ok(())
}
