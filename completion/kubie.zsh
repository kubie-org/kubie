#compdef kubie

function _kubie {
    local -a subcmds
    local context state line

    _arguments -C \
        '1: :->param1' \
        '2: :->param2' \
        '3: :->param3' && return 0

    case $state in
        param1)
            subcmds=('ctx' 'edit' 'edit-config' 'exec' 'help' 'info' 'lint' 'ns')
            _describe 'command' subcmds
            ;;
        param2)
            case $line[1] in
                ctx|edit|exec)
                    subcmds=(${(f)"$(kubie ctx)"})
                    _describe 'context' subcmds
                    ;;
                ns)
                    subcmds=(${(f)"$(kubie ns)"})
                    _describe 'namespace' subcmds
                    ;;
            esac
            ;;
        param3)
            if [[ $line[1] == 'exec' ]]; then
                subcmds=(${(f)"$(kubie exec $line[2] default kubectl get namespaces | tail -n+2 | awk '{print $1}')"})
                _describe 'namespace' subcmds
            fi
            ;;
    esac
}

compdef _kubie kubie
