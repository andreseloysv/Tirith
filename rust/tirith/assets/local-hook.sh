
local_hook="$(cd "$(dirname "$0")" && pwd)/$(basename "$0").local"
if [ -x "$local_hook" ]; then
    "$local_hook" "$@" || exit 1
fi
