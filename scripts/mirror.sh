#!/usr/bin/env bash
# Mirror this repository to all three forges.
#
# The URLs are overridable because they will drift: override with
#   FELIX_GH=... FELIX_CB=... FELIX_G8T=... scripts/mirror.sh
#
# `ssh` is the transport the operator uses for Codeberg and g8t (see
# ~/.ssh/config), so the defaults are scp-style. GitHub also has an https path
# through `gh`, which is why it is listed first: it is the one that works from a
# fresh machine with nothing but `gh auth login`.
set -euo pipefail

GH="${FELIX_GH:-git@github.com:8b-is/felix.git}"
CB="${FELIX_CB:-git@codeberg.org:lodripeter-sec/felix.git}"
G8T="${FELIX_G8T:-git@g8t.8b.is:8b-is/felix.git}"
BRANCH="${FELIX_BRANCH:-main}"
REMOTE="all"

say() { printf '  %s\n' "$*"; }

cd "$(git rev-parse --show-toplevel)"

if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "refusing to mirror a dirty tree; commit first" >&2
    exit 1
fi

echo "felix mirror · branch $BRANCH"
say "github   $GH"
say "codeberg $CB"
say "g8t      $G8T"

# Reachability first. A mirror that half-runs leaves the forges disagreeing,
# which is worse than not trying.
fail=0
for pair in "github $GH" "codeberg $CB" "g8t $G8T"; do
    set -- $pair
    name=$1
    url=$2
    if git ls-remote --exit-code "$url" >/dev/null 2>&1; then
        say "$name: reachable"
    else
        say "$name: UNREACHABLE (repo missing, or no credentials on this host)"
        fail=1
    fi
done

if [ "$fail" -ne 0 ]; then
    cat >&2 <<'EOF'

Not pushing: at least one forge is not reachable from here.

Create the missing remotes first, then re-run this script:

  codeberg   https://codeberg.org/repo/create   (owner: lodripeter-sec, name: felix)
             or, with a token:
             curl -X POST https://codeberg.org/api/v1/user/repos \
               -H "Authorization: token $CODEBERG_TOKEN" \
               -H 'Content-Type: application/json' \
               -d '{"name":"felix","private":false,"description":"Felix v1.0 — linear values, exact p/q scales, ternary words"}'

  g8t        Forgejo at g8t.8b.is — create `8b-is/felix` there, then ensure the
             host alias in ~/.ssh/config resolves for this machine.
EOF
    exit 1
fi

git remote remove "$REMOTE" 2>/dev/null || true
git remote add "$REMOTE" "$GH"
git remote set-url --add --push "$REMOTE" "$GH"
git remote set-url --add --push "$REMOTE" "$CB"
git remote set-url --add --push "$REMOTE" "$G8T"

echo
echo "pushing to three forges at once..."
git push "$REMOTE" "$BRANCH"

echo
echo "verify (each line must print the same commit):"
for pair in "github $GH" "codeberg $CB" "g8t $G8T"; do
    set -- $pair
    printf '  %-9s ' "$1"
    git ls-remote "$2" "refs/heads/$BRANCH" | awk '{print $1}'
done
printf '  %-9s ' "local"
git rev-parse HEAD
