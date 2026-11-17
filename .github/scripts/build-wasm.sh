#!/usr/bin/env bash
# Build Dioxus WASM chart apps and collect their web bundles into dist/<app>/.
#
# Usage: .github/scripts/build-wasm.sh <app> [<app> ...]
#
# Every app is attempted even if an earlier one fails, and the script exits
# non-zero if any failed, so a broken chart can never be published silently.
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <app> [<app> ...]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DIST="${DIST_DIR:-dist}"
mkdir -p "$DIST"

built=()
failed=()

build_app() {
  local app="$1"
  local public_dir="target/dx/${app}/release/web/public"

  # dx never prunes old content-hashed bundles; start clean so exactly one
  # JS/WASM pair is published per app.
  rm -rf "$public_dir"
  (cd "$app" && dx build --release --platform web) || return 1

  # A usable bundle has the JS loader and the wasm-bindgen binary.
  if ! compgen -G "${public_dir}/assets/*.js" > /dev/null ||
     ! compgen -G "${public_dir}/assets/*_bg*.wasm" > /dev/null; then
    echo "::error::${app}: build finished but ${public_dir}/assets has no JS/WASM bundle"
    return 1
  fi

  rm -rf "${DIST:?}/${app}"
  mkdir -p "${DIST}/${app}"
  cp -R "${public_dir}/." "${DIST}/${app}/"
  # The blog embeds the app itself; dx's standalone page is not needed.
  rm -f "${DIST}/${app}/index.html"
}

for app in "$@"; do
  echo "::group::Build ${app}"
  if build_app "$app"; then
    built+=("$app")
  else
    echo "::error::Build failed for ${app}"
    failed+=("$app")
  fi
  echo "::endgroup::"
done

echo "Built (${#built[@]}): ${built[*]:-none}"
echo "Failed (${#failed[@]}): ${failed[*]:-none}"
find "$DIST" -type f | sort

[ "${#failed[@]}" -eq 0 ]
