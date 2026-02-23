# Kubie
> [!IMPORTANT]
> Please see issue [#385](https://github.com/sbstp/kubie/issues/385) regarding the future of Kubie development.

<img src="./assets/logo.svg" align="right"/>

`kubie` is an alternative to `kubectx`, `kubens` and the `k on` prompt modification script. It offers context switching,
namespace switching and prompt modification in a way that makes each shell independent from others. It also has
support for split configuration files, meaning it can load Kubernetes contexts from multiple files. You can configure
the paths where kubie will look for contexts, see the [settings](#settings) section.

Kubie also has other nice features such as `kubie exec` which allows you to execute commands in a context and a
namespace without having to spawn a shell and `kubie lint` which scans your k8s config files for issues and informs
you of what they are.

* [Installation](#installation)
* [Usage](#usage)
* [Settings](#settings)
* [Future plans](#future-plans)

Thanks to [@ahermant](https://github.com/ahermant) for the lovely logo!

## Installation

### Binary
You can download a binary for Linux or OS X on the [GitHub releases page](https://github.com/sbstp/kubie/releases). You
can use `curl` or `wget` to download it. Don't forget to `chmod +x` the file!

### Cargo
You can build `kubie` from source using `cargo` and crates.io. If you do not have a Rust compiler installed, go to
[rustup.rs](https://rustup.rs) to get one. Then you can run `cargo install kubie` and kubie will be downloaded from
crates.io and then built.

### Homebrew
You can install `kubie` from Homebrew by running `brew install kubie`.

### MacPorts
You can also install `kubie` from [MacPorts](https://www.macports.org) by running `sudo port install kubie`.

### Nix
There is a `kubie` Nix package maintained by @illiusdope that you can install.

### Arch Linux
`kubie` is available in the [extra repository](https://archlinux.org/packages/extra/x86_64/kubie/), and it can be installed by running `pacman -S kubie`.

### Autocompletion

Autocompletion can be enabled by adding one of the following lines to your shell's configuration file, e.g., `~/.bashrc`
or `~/.zshrc`:

```sh
# Bash / Zsh / others
source <(kubie generate-completion)
# Fish
kubie generate-completion fish | source
```

See the [clap-complete docs](https://docs.rs/clap_complete/latest/clap_complete/aot/enum.Shell.html) for all supported
shells.

## Usage
Selectable menus will be available when using `kubie ctx` and `kubie ns`.

---

* `kubie ctx` display a selectable menu of contexts
* `kubie ctx <context>` switch the current shell to the given context (spawns a shell if not a kubie shell)
* `kubie ctx -` switch back to the previous context within current shell session
* `kubie ctx --last-used` switch to the last used context (relies on `track_last_used` config option)
* `kubie ctx <context> -r` spawn a recursive shell in the given context
* `kubie ctx <context> -n <namespace>` spawn a shell in the given context and namespace
* `kubie ns` display a selectable menu of namespaces
* `kubie ns <namespace>` switch the current shell to the given namespace
* `kubie ns -` switch back to the previous namespace within current shell session
* `kubie ns <namespace> -r` spawn a recursive shell in the given namespace
* `kubie exec <context> <namespace> <cmd> <args>...` execute a command in the given context and namespace
* `kubie exec <wildcard> <namespace> <cmd> <args>...` execute a command in all the contexts matched by the wildcard and
  in the given namespace
* `kubie exec <wildcard> <namespace> -e <cmd> <args>...` execute a command in all the contexts matched by the wildcard and
  in the given namespace but fail early if any of the commands executed return a non-zero exit code
* `kubie export <context> <namespace>` prints the path to an isolated config file for a context and namespace
* `kubie edit` display a selectable menu of contexts to edit
* `kubie edit <context>` edit the file that contains this context
* `kubie edit-config` edit kubie's own config file
* `kubie lint` lint k8s config files for issues
* `kubie info ctx` print name of current context
* `kubie info ns` print name of current namespace
* `kubie info depth` print depth of recursive contexts
* `kubie update` will check the latest kubie version and update your local installation if needed

## Settings
You can customize kubie's behavior with the `~/.kube/kubie.yaml` file. The settings available and their defaults are
available below.

```yaml
# Force kubie to use a particular shell, if unset detect shell currently in use.
# Possible values: bash, dash, fish, xonsh, zsh
# Default: unset
shell: bash

# For the commands `kubie edit/edit-config`
# Possible values: Any installed text editor
# Default: unset
default_editor: vim

# Configure where to look for kubernetes config files.
configs:

    # Include these globs.
    # Default: values listed below.
    include:
        - ~/.kube/config
        - ~/.kube/*.yml
        - ~/.kube/*.yaml
        - ~/.kube/configs/*.yml
        - ~/.kube/configs/*.yaml
        - ~/.kube/kubie/*.yml
        - ~/.kube/kubie/*.yaml

    # Exclude these globs.
    # Default: values listed below.
    # Note: kubie's own config file is always excluded.
    exclude:
        - ~/.kube/kubie.yaml

# Prompt settings.
prompt:
    # Disable kubie's custom prompt inside of a kubie shell. This is useful
    # when you already have a prompt displaying kubernetes information.
    # Default: false
    disable: false

    # When using recursive contexts, show depth when larger than 1.
    # Default: true
    show_depth: true

    # When using zsh, show context and namespace on the right-hand side using RPS1.
    # Default: false
    zsh_use_rps1: false

    # When using fish, show context and namespace on the right-hand side.
    # Default: false
    fish_use_rprompt: false

    # When using xonsh, show context and namespace on the right-hand side.
    # Default: false
    xonsh_use_right_prompt: false

# Behavior
behavior:
    # Namespace validation and switching behavior.  Set to "false" if you do not have
    # the right to list namespaces.
    # Valid values:
    #   true:    Make sure the namespace exists with `kubectl get namespaces`.
    #   false:   Switch namespaces without validation.
    #   partial: Check for partial matches when running `kubie ns <namespace>`
    #            and no exact match is found:
    #              - if exactly one namespace partially matches, switch to that namespace
    #              - if multiple namespaces partially match, select from those
    # Default: true
    validate_namespaces: true

    # Enable or disable the printing of the 'CONTEXT => ...' headers when running
    # `kubie exec`.
    # Valid values:
    #   auto:   Prints context headers only if stdout is a TTY. Piping/redirecting
    #           kubie output will auto-disable context headers.
    #   always: Always prints context headers, even if stdout is not a TTY.
    #   never:  Never prints context headers.
    # Default: auto
    print_context_in_exec: auto

    # Parse the CONTEXT_NAME argument to `kubie exec` and `kubie export` as a
    # space-delimited list, allowing multiple patterns to be specified.
    # Example:
    #   kubie exec 'dev-* pre-* staging-1 prod-2' kube-system -- kubectl get po
    # Default: false
    allow_multiple_context_patterns: false

    # Track the last used context globally and allow restoring it with
    # `kubie ctx --last-used`. This persists across terminal sessions.
    # Default: false
    track_last_used: false

# Optional start and stop hooks
hooks:
    # A command hook to run when a CTX is started.  
    # This example re-labels your terminal window
    # Default: none
    start_ctx: >
        echo -en "\033]1; `kubie info ctx`|`kubie info ns` \007"

    # A command hook to run when a CTX is stopped
    # This example sets the terminal back to the shell name
    # Default: none
    stop_ctx: >
        echo -en "\033]1; $SHELL \007"

```

## For distro maintainers
Since `0.19.0`, the self update functionality is behind a feature. You can use `cargo build --release --no-default-features`
to produce a binary without the self update functionality. It's probably better if people rely on the distro's package
manager for updates over this functionality. The binary produced is also quite smaller since it has fewer dependencies.

## Future plans
* Integration with vault to automatically download k8s configs from a vault server
* Import/edit configs
