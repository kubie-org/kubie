#Kubie completion script

_kubiecomplete()
{
    local cur prev

    cur=${COMP_WORDS[COMP_CWORD]}
    prev=${COMP_WORDS[COMP_CWORD-1]}

    { \unalias command; \unset -f command; } >/dev/null 2>&1 || true

    case ${COMP_CWORD} in
        1)
            cmds="ctx edit edit-config exec help info lint ns"
            COMPREPLY=($(command printf "%s\n" $cmds | command grep -e "^$cur" | command xargs))
            ;;
        2)
            case ${prev} in
                ctx)
                    COMPREPLY=($(command kubie ctx | command grep -e "^$cur" | command xargs))
                    ;;
                edit)
                    COMPREPLY=($(command kubie ctx | command grep -e "^$cur" | command xargs))
                    ;;
                exec)
                    COMPREPLY=($(command kubie ctx | command grep -e "^$cur" | command xargs))
                    ;;
                ns)
                    COMPREPLY=($(command kubie ns | command grep -e "^$cur" | command xargs))
                    ;;
            esac
            ;;
        3)
            prevprev=${COMP_WORDS[COMP_CWORD-2]}
            case ${prevprev} in
                exec)
                    COMPREPLY=($(command kubie exec ${prev} default kubectl get namespaces|command tail -n+2|command awk '{print $1}'| command grep -e "^$cur" |command xargs))
                    ;;
            esac
            ;;
        *)
            COMPREPLY=()
            ;;
    esac
}

complete -F _kubiecomplete kubie
