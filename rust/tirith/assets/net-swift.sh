case "$1" in build|test|run|package)
  case " $* " in *" --disable-sandbox "*) exec swift "$@";; esac
  sub=$1; shift; exec swift "$sub" --disable-sandbox "$@";;
esac
exec swift "$@"
