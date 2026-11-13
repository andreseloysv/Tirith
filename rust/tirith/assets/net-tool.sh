case " $* " in *" --disable-sandbox "*) exec {tool} "$@";; esac
exec {tool} --disable-sandbox "$@"
