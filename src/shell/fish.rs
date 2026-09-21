use std::process::Command;

use anyhow::Result;

use super::ShellSpawnInfo;

pub fn spawn_shell(info: &ShellSpawnInfo) -> Result<()> {
    let mut cmd = Command::new("fish");
    // run fish as an interactive login shell
    cmd.arg("-ilC");
    cmd.arg(format!(
        r#"
# Set the proper KUBECONFIG variable before each command runs,
# to prevent the user from overwriting it.
function kubie_preexec --on-event fish_preexec
    set -xg KUBECONFIG "$KUBIE_KUBECONFIG"
end

if test "$KUBIE_PROMPT_DISABLE" = "0"
    # The general idea behind the prompt substitions is to save the existing
    # prompt's output _before_ anything else is run. This is important since the
    # existing prompt might be dependent on say the status of the executed command.

    if test "$KUBIE_FISH_USE_RPROMPT" = "1"
        functions -q fish_right_prompt
        and functions --copy fish_right_prompt fish_right_prompt_original
        or function fish_right_prompt_original; end

        function fish_right_prompt
            set -l original (fish_right_prompt_original)

            # Fish's right prompt does not support newlines, so there's no point in
            # iterating through the (potentially) existing prompt's lines.
            printf '%s %s' (string unescape {prompt}) $original
        end
    else
        functions --copy fish_prompt fish_prompt_original
        function fish_prompt
            set -l original (fish_prompt_original)

            printf '%s ' (string unescape {prompt})

            # Due to idiosyncrasies with the way fish is managing newlines in
            # process substitions, each line needs to be printed separately
            # to mirror the existing output. For more details,
            # see https://github.com/fish-shell/fish-shell/issues/159.
            for line in $original
                echo -e $line
            end
        end
    end
end
    "#,
        prompt = info.prompt,
    ));
    info.env_vars.apply(&mut cmd);

    let mut child = cmd.spawn()?;
    child.wait()?;
    Ok(())
}

pub fn key_bindings() -> &'static str {
    include_str!("../../shell/key-bindings/kubie.fish")
}
