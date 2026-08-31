#!/bin/sh
# crates/baton-term may diverge from iced_term, but every divergence must be
# recorded: regenerating the record from the published crate has to reproduce
# the diff body of UPSTREAM.diff byte for byte. The recorded format, measured:
# labels are `iced_term-0.8.0/src/<f>` vs `baton-term/src/<f>`, and a file
# with no upstream counterpart is a three-line stub, not its full content.
set -eu
# Pinned so the union's sort order cannot depend on the runner's locale.
export LC_ALL=en_US.UTF-8
cd "$(dirname "$0")/../.."

ver=0.8.0   # the hardcopy's base; UPSTREAM.diff's header names it
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl -sSfL --retry 3 -o "$work/iced_term.crate" \
    "https://static.crates.io/crates/iced_term/iced_term-$ver.crate" \
    || { echo "baton-term: download failed -- NOT a freeze violation" >&2; exit 1; }
tar xzf "$work/iced_term.crate" -C "$work"

# Walk the UNION of both trees, not just the copy's files: a file deleted from
# the copy is a divergence too, and one the copy-only glob was blind to.
{ (cd "$work/iced_term-$ver/src" && find . -name '*.rs')
  (cd crates/baton-term/src && find . -name '*.rs'); } \
  | sed 's|^\./||' | sort -u > "$work/union"

while read -r rel; do
    up="$work/iced_term-$ver/src/$rel"
    ours="crates/baton-term/src/$rel"
    if [ -f "$up" ] && [ -f "$ours" ]; then
        diff -u "$up" "$ours" \
          | sed -e "1s|^--- .*|--- iced_term-$ver/src/$rel|" \
                -e "2s|^+++ .*|+++ baton-term/src/$rel|"
    elif [ -f "$ours" ]; then
        printf -- '--- /dev/null\n+++ baton-term/src/%s\n(new file, ours entirely)\n' "$rel"
    else
        # Deleted from the copy. The record has no spelling for a deletion, so
        # this line can never match UPSTREAM.diff -- which is the point.
        printf -- '--- iced_term-%s/src/%s\n+++ /dev/null\n(deleted from the copy, unrecorded)\n' "$ver" "$rel"
    fi
done < "$work/union" > "$work/regen" || true

awk '/^--- /{found=1} found' crates/baton-term/UPSTREAM.diff > "$work/recorded"

if ! diff -q "$work/regen" "$work/recorded" > /dev/null; then
    echo "baton-term: the copy's divergence from iced_term-$ver no longer matches UPSTREAM.diff." >&2
    echo "Regenerate it per the header of crates/baton-term/UPSTREAM.diff." >&2
    diff "$work/regen" "$work/recorded" >&2 || true
    exit 1
fi
echo "hardcopy-term: divergence from iced_term-$ver matches UPSTREAM.diff"
