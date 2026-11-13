
# In a merge, what a parent already has is not the merge's doing: only what the merge adds is checked.
export TIRITH_MERGE_HEAD=$(git rev-parse -q --verify MERGE_HEAD 2>/dev/null || true)
git diff --cached --no-color --no-ext-diff -U0 --diff-filter=ACMR | perl -ne '
    BEGIN { $re = qr/(?<![A-Za-z0-9])($ENV{TIRITH_GUARD_RE})(?![A-Za-z0-9])/i; $bad = 0; $mh = $ENV{TIRITH_MERGE_HEAD}; %seen = () }
    if (/^\+\+\+ b\/(.*)$/) {
        $file = $1;
        %seen = ();
        $inparent = 0;
        if ($mh) {
            my $out = `git show \Q$mh\E:\Q$file\E 2>/dev/null`;
            if ($? == 0) { $inparent = 1; for my $l (split /\n/, $out, -1) { $seen{$l} = 1 } }
        }
        if (!$inparent && $file =~ $re) { print STDERR "  $file: \"$1\" in the file name\n"; $bad = 1 }
        next;
    }
    if (/^@@ -\S+ \+(\d+)/) { $line = $1; next }
    if (/^\+/) {
        my $text = substr($_, 1); chomp $text;
        if (!$seen{$text} && /$re/) { chomp; my $t = substr($_, 1, 120); print STDERR "  $file:$line: \"$1\" in: $t\n"; $bad = 1 }
        $line++;
    }
    END {
        if ($bad) { print STDERR "\npre-commit: commit blocked, forbidden words in the staged changes (see above).\n"; exit 1 }
    }
'
