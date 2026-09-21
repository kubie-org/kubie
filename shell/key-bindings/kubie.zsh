#!/bin/zsh

# zsh key bindings for kubie

[[ -o interactive ]] || return
command -v kubie >/dev/null 2>&1 || return
zmodload zsh/zle 2>/dev/null || true

__kubie_run() {
  zle -I
  command "$@" < /dev/tty
  local ret=$?

  zle reset-prompt
  return $ret
}

__kubie_bind_all() {
  local key="$1" widget="$2" km
  for km in emacs viins vicmd; do
    bindkey -M "$km" "$key" "$widget" 2>/dev/null || true
  done
}

__kubie_bind() {
  local var="$1" def="$2" widget="$3"
  [[ "${(P)var-x}" != "" ]] || return 0
  local key="${(P)var-$def}"
  __kubie_bind_all "$key" "$widget"
}

kubie-ctx-widget()      { __kubie_run kubie ctx }
kubie-ns-widget()       { __kubie_run kubie ns }
kubie-prev-ctx-widget() { __kubie_run kubie ctx - }
kubie-prev-ns-widget()  { __kubie_run kubie ns - }

zle -N kubie-ctx-widget
zle -N kubie-ns-widget
zle -N kubie-prev-ctx-widget
zle -N kubie-prev-ns-widget

__kubie_bind KUBIE_CTX_KEY      '\ek' kubie-ctx-widget
__kubie_bind KUBIE_NS_KEY       '\en' kubie-ns-widget
__kubie_bind KUBIE_PREV_CTX_KEY '\eK' kubie-prev-ctx-widget
__kubie_bind KUBIE_PREV_NS_KEY  '\eN' kubie-prev-ns-widget

unset -f __kubie_bind_all __kubie_bind
