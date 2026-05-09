#!/bin/bash

# bash key bindings for kubie

[[ $- == *i* ]] || return
command -v kubie >/dev/null 2>&1 || return

__kubie_run() {
  command "$@" < /dev/tty
}

__kubie_ctx_widget() {
  __kubie_run kubie ctx
}

__kubie_ns_widget() {
  __kubie_run kubie ns
}

__kubie_prev_ctx_widget() {
  __kubie_run kubie ctx -
}

__kubie_prev_ns_widget() {
  __kubie_run kubie ns -
}

__kubie_bind() {
  local var="$1" def="$2" func="$3"
  local key="${!var}"
  if [[ -n "${!var+x}" && -z "$key" ]]; then
    return 0
  fi
  key="${key:-$def}"
  bind -x "\"$key\":$func"
  bind -m vi-insert -x "\"$key\":$func" 2>/dev/null
  bind -m vi-command -x "\"$key\":$func" 2>/dev/null
}

__kubie_bind KUBIE_CTX_KEY      '\ek' __kubie_ctx_widget
__kubie_bind KUBIE_NS_KEY       '\en' __kubie_ns_widget
__kubie_bind KUBIE_PREV_CTX_KEY '\eK' __kubie_prev_ctx_widget
__kubie_bind KUBIE_PREV_NS_KEY  '\eN' __kubie_prev_ns_widget

unset -f __kubie_bind
