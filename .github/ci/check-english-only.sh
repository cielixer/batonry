#!/bin/sh
# Every character git tracks is English (CLAUDE.md section 5). Files whose
# subject IS the text -- IME fixtures, upstream sources, quoted evidence, and
# the two files that contain the character class this check greps for -- are
# enumerated in .github/korean-allowlist.txt, which is the only enumeration:
# a difference in EITHER direction fails, because an extra file is a violation
# and a missing one means the allowlist has gone over-permissive.
set -eu
# The bracket class below is a Hangul range only in a UTF-8 locale; under
# LC_ALL=C it degrades to byte ranges and matches any non-ASCII byte, turning
# a clean tree into ~30 false hits. CI runners do not guarantee a locale, so
# pin it -- this also pins sort's collation on both sides of the comparison.
export LC_ALL=en_US.UTF-8
cd "$(dirname "$0")/../.."

actual=$(git ls-files -z | xargs -0 grep -l '[가-힣]' | sort)
expected=$(sort .github/korean-allowlist.txt)

if [ "$actual" != "$expected" ]; then
    echo "Korean-character allowlist mismatch." >&2
    echo "--- files containing Hangul now:" >&2
    echo "$actual" >&2
    echo "--- .github/korean-allowlist.txt:" >&2
    echo "$expected" >&2
    echo "An extra file is an English-only violation; a missing one means the" >&2
    echo "allowlist is over-permissive and must shrink in the same commit." >&2
    exit 1
fi
echo "english-only: allowlist matches ($(printf '%s\n' "$expected" | wc -l | tr -d ' ') files)"
