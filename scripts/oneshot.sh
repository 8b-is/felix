#!/usr/bin/env bash
# The whole v1.0 bill, one command:
#
#   scripts/oneshot.sh            # format, lint, unit + E2E tests, capsules, oracle
#   FELIX_STRICT=1 scripts/oneshot.sh   # also fail if the memnet oracle is dirty
#
# Every step is a deterministic external check; nothing here is a judgement call.
# Ends with one line: green, or the step that failed.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
step() { printf '\n▸ %s\n' "$*"; }
trap 'printf "\n✗ oneshot failed at: %s\n" "${current:-setup}" >&2' ERR

current="fmt";      step "$current";      cargo fmt -p felix-lang -p felix-cli --check   # not --all: that reformats ../memnet
current="clippy";   step "$current";      cargo clippy -q --all-targets -- -D warnings
current="test";     step "$current (unit + E2E goldens)"
cargo test -q 2>&1 | grep -E '^test result|FAILED|panicked' || true
cargo test -q >/dev/null 2>&1

current="build";    step "$current";      cargo build -q
felix=target/debug/felix

current="examples"; step "$current"
for flx in examples/*.flx; do
    name="${flx%.flx}"
    if [ -f "$name.err" ]; then
        if "$felix" "$flx" >/dev/null 2>&1; then
            echo "  $flx should have been refused" >&2; false
        fi
        printf '  %-36s refused, as intended\n' "$flx"
    else
        diff -u "$name.out" <("$felix" "$flx")
        printf '  %-36s %s lines, matches golden\n' "$flx" "$(wc -l <"$name.out" | tr -d ' ')"
    fi
done

current="capsules"; step "$current"
for capsule in capsules/*.flxc; do
    printf '  %-36s ' "$capsule"
    "$felix" verify "$capsule"
done

# Felix links ../memnet by path on purpose (AGENTS.md: the oracle wins). That
# also means a clean felix checkout is only reproducible if memnet's tree is.
current="oracle";   step "$current (../memnet)"
oracle_dirty="$(git -C ../memnet status --porcelain -- src 2>/dev/null || echo "unreadable")"
if [ -n "$oracle_dirty" ]; then
    echo "  WARNING: the memnet oracle has uncommitted changes Felix builds against:"
    printf '%s\n' "$oracle_dirty" | sed 's/^/    /'
    echo "  A fresh clone of felix + memnet will not build until these are committed."
    if [ "${FELIX_STRICT:-0}" = "1" ]; then false; fi
else
    echo "  clean at $(git -C ../memnet rev-parse --short HEAD)"
fi

trap - ERR
printf '\n✓ felix %s · oneshot green\n' "$("$felix" --version | cut -d' ' -f2)"
