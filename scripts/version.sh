#!/bin/sh
# Single source of the version: `version` in [workspace.package] of Cargo.toml.
# Run it to print the version, or source it from the repository root to get $version.
case "$0" in */version.sh) cd "$(dirname "$0")/.." ;; esac
version=$(awk -F'"' '/^\[/ { s = ($0 == "[workspace.package]") } s && /^version *=/ { print $2; exit }' Cargo.toml)
[ -n "$version" ] || { echo "Version not found in Cargo.toml" >&2; exit 1; }
case "$0" in */version.sh) printf '%s\n' "$version" ;; esac
