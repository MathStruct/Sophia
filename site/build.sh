#!/usr/bin/env bash
# Build this repository's website with MathStruct's shared Quartz
# (https://github.com/MathStruct/quartz: Quartz plus the plugins and fixes all MathStruct sites
# share). Needs git and Node >= 22.
#
#   site/build.sh            # -> site/public/
#   site/build.sh --serve    # live preview on http://localhost:8080, rebuilt as you edit
#
# The shared Quartz is cloned into site/.quartz/ (gitignored) and moved to QUARTZ_REF on every
# build. To try a change to the shared Quartz on this site before pushing it, point QUARTZ_DIR
# at your checkout of it: QUARTZ_DIR=~/Code/MathStruct-quartz site/build.sh --serve
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTENT="$HERE/.."
QUARTZ_REF="${QUARTZ_REF:-main}"

ENGINE="${QUARTZ_DIR:-$HERE/.quartz}"
if [[ -z "${QUARTZ_DIR:-}" ]]; then
  [[ -d "$ENGINE/.git" ]] || git clone -q --depth 1 https://github.com/MathStruct/quartz.git "$ENGINE"
  if git -C "$ENGINE" fetch -q --depth 1 origin "$QUARTZ_REF"; then
    git -C "$ENGINE" checkout -q -f --detach FETCH_HEAD
  else
    echo ">> offline: building with site/.quartz as it is ($(git -C "$ENGINE" rev-parse --short HEAD))"
  fi
fi
exec "$ENGINE/mathstruct/site.sh" "$HERE" "$CONTENT" "$@"
