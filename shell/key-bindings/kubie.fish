#!/usr/bin/env fish

# fish key bindings for kubie

status is-interactive; or exit
command -q kubie; or exit

function __kubie_ctx_widget
    commandline -f repaint
    command kubie ctx </dev/tty
    commandline -f repaint
end

function __kubie_ns_widget
    commandline -f repaint
    command kubie ns </dev/tty
    commandline -f repaint
end

function __kubie_prev_ctx_widget
    commandline -f repaint
    command kubie ctx - </dev/tty
    commandline -f repaint
end

function __kubie_prev_ns_widget
    commandline -f repaint
    command kubie ns - </dev/tty
    commandline -f repaint
end

function __kubie_bind
    set -l var $argv[1]
    set -l def $argv[2]
    set -l func $argv[3]

    if set -q $var; and test -z "$$var"
        return 0
    end

    if set -q $var
        set -l key $$var
        bind --preset $key $func
        bind -M insert $key $func 2>/dev/null
    else
        bind --preset $def $func
        bind -M insert $def $func 2>/dev/null
    end
end

__kubie_bind KUBIE_CTX_KEY \ek __kubie_ctx_widget
__kubie_bind KUBIE_NS_KEY \en __kubie_ns_widget
__kubie_bind KUBIE_PREV_CTX_KEY \eK __kubie_prev_ctx_widget
__kubie_bind KUBIE_PREV_NS_KEY \eN __kubie_prev_ns_widget

functions -e __kubie_bind
