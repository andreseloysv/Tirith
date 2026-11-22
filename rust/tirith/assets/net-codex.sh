real=$(command -v codex) || { echo "codex: not found" >&2; exit 127; }
TIRITH_HOME="{home}" exec "{exe}" nested-codex "$real" "$@"
