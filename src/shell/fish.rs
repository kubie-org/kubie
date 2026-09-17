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
        prompt = escape_prompt(&info.prompt),
    ));
    info.env_vars.apply(&mut cmd);

    let mut child = cmd.spawn()?;
    child.wait()?;
    Ok(())
}

/// Escape the prompt so fish reads it as one word when it is spliced into the init script above.
///
/// The prompt looks like `[\e[31m(kubie info ctx)\e[0m|\e[32m(kubie info ns)\e[0m]`. In an unquoted
/// fish word, `[` starts a slice and `|` starts a new command. Fish 4.8 and earlier happened to read the
/// whole prompt as one word, because the first `[` switched the tokenizer into a mode where the `|` was
/// ignored until the closing `]`. Fish 4.9 counts bracket nesting instead, so the five `[` never balance
/// the single `]` and the whole init script fails to parse with "Missing end to balance this function
/// definition", even when the prompt is disabled.
///
/// Escaping the brackets and the pipe makes the old behaviour explicit and works on every fish version.
/// Only these three characters are escaped because the rest of the prompt has to stay live: the
/// `(...)` command substitutions and the `\e` escapes are expanded by fish. Quoting the prompt would not
/// work either, since fish only expands `$(cmd)`, not `(cmd)`, inside double quotes.
fn escape_prompt(prompt: &str) -> String {
    let mut escaped = String::with_capacity(prompt.len() + 8);
    for c in prompt.chars() {
        if matches!(c, '[' | ']' | '|') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

#[test]
fn test_escape_prompt() {
    assert_eq!(
        escape_prompt(r"[\e[31m(kubie info ctx)\e[0m|\e[32m(kubie info ns)\e[0m]"),
        r"\[\e\[31m(kubie info ctx)\e\[0m\|\e\[32m(kubie info ns)\e\[0m\]"
    );
}

#[test]
fn test_escape_prompt_leaves_other_characters_alone() {
    assert_eq!(escape_prompt(r"(kubie info ctx) \e[0m"), r"(kubie info ctx) \e\[0m");
}
