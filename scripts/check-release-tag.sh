#!/bin/sh

set -eu

tag=${1:-${GITHUB_REF_NAME:-}}
[ -n "$tag" ] || { printf '%s\n' "release tag is required" >&2; exit 1; }

printf '%s\n' "$tag" | grep -Eq '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$' || {
    printf "invalid release tag: %s\n" "$tag" >&2
    exit 1
}

version=$(awk '
    /^\[package\]$/ { inPackage=1; next }
    /^\[/ { inPackage=0 }
    inPackage && /^version[[:space:]]*=/ { gsub(/[[:space:]\"]/, "", $0); sub(/^version=/, "", $0); print; exit }
' crates/codegen/xai-grok-pager-bin/Cargo.toml)

[ "$tag" = "v$version" ] || {
    printf "release tag %s does not match pager binary version %s\n" "$tag" "$version" >&2
    exit 1
}

printf "release tag %s matches echo-build %s\n" "$tag" "$version"
