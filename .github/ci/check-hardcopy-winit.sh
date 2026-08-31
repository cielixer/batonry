#!/bin/sh
# crates/winit's src/ is FROZEN: it differs from the published winit only by
# the macOS IME patch, and every divergent line is recorded in UPSTREAM.diff.
# This regenerates that record from the published crate and requires it to
# match byte for byte -- which catches both a drift in the copy and an edit
# nobody recorded. Scope is src/ deliberately: the manifest diverges on
# purpose (trimmed examples and tests) and that divergence is not recorded.
# The version is read from the copy's own Cargo.toml.
set -eu
cd "$(dirname "$0")/../.."

ver=$(sed -n '/^\[package\]/,/^\[/s/^version = "\(.*\)"/\1/p' crates/winit/Cargo.toml | head -1)
[ -n "$ver" ] || { echo "winit: could not read [package].version" >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl -sSfL --retry 3 -o "$work/winit.crate" \
    "https://static.crates.io/crates/winit/winit-$ver.crate" \
    || { echo "winit: download failed -- NOT a freeze violation" >&2; exit 1; }
tar xzf "$work/winit.crate" -C "$work"

# The whole divergence must be the files UPSTREAM.diff records -- one today.
diff -rq "$work/winit-$ver/src" crates/winit/src > "$work/files" || true
recorded=$(grep -c '^--- ' crates/winit/UPSTREAM.diff || true)
[ "$recorded" -gt 0 ] || { echo "winit: UPSTREAM.diff records no files -- is it intact?" >&2; exit 1; }
actual=$(wc -l < "$work/files" | tr -d ' ')
if [ "$actual" != "$recorded" ]; then
    echo "winit: $actual files differ from winit-$ver but UPSTREAM.diff records $recorded:" >&2
    cat "$work/files" >&2
    exit 1
fi

grep '^--- ' crates/winit/UPSTREAM.diff | sed 's|^--- winit-[^/]*/||' \
| while read -r rel; do
    diff -u "$work/winit-$ver/$rel" "crates/winit/$rel" \
      | sed -e "1s|^--- .*|--- winit-$ver/$rel|" \
            -e "2s|^+++ .*|+++ crates/winit/$rel|"
done > "$work/regen" || true

if ! diff -q "$work/regen" crates/winit/UPSTREAM.diff > /dev/null; then
    echo "winit: the copy's divergence from winit-$ver no longer matches UPSTREAM.diff:" >&2
    diff "$work/regen" crates/winit/UPSTREAM.diff >&2 || true
    exit 1
fi
echo "hardcopy-winit: divergence from winit-$ver matches UPSTREAM.diff"
