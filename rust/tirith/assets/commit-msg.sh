
# A lane commit's default attribution trailer is dropped, not blocked.
if [ -n "${TIRITH_LANE:-}" ] && [ -f "${1:-}" ]; then
    tirith_msg=$(mktemp "${TMPDIR:-/tmp}/tirith-msg.XXXXXX") || tirith_msg=
    if [ -n "$tirith_msg" ]; then
        { grep -v -i -E '^[[:space:]]*co-[a-z]+-by:' "$1" || true; } > "$tirith_msg"
        cat "$tirith_msg" > "$1"
        rm -f "$tirith_msg"
    fi
fi
perl -ne '
    BEGIN { $re = qr/(?<![A-Za-z0-9])($ENV{TIRITH_GUARD_RE})(?![A-Za-z0-9])/i; $bad = 0 }
    next if /^#/;
    if (/$re/) { chomp; print STDERR "  line $.: \"$1\" in: $_\n"; $bad = 1 }
    END {
        if ($bad) { print STDERR "\ncommit-msg: commit blocked, forbidden words in the commit message (see above).\n"; exit 1 }
    }
' "$1"
