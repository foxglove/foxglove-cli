#!/usr/bin/env bash
# Write release notes to stdout; the v2 launch includes all changes since v1.0.33.
set -euo pipefail

release_tag=${1:?release tag is required}
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
case "$release_tag" in
  v2.0.0|v2.0.0-rc.*)
    notes=$(cat docs/v2-release-notes.md)
    notes=${notes/"# Foxglove CLI v2.0.0"/"# Foxglove CLI $release_tag"}
    migration_path="blob/main/docs/v2-migration.md"
    tagged_path="blob/$release_tag/docs/v2-migration.md"
    notes=${notes//$migration_path/$tagged_path}
    printf '%s\n' "$notes"
    ;;
  *) git log --oneline --no-merges --first-parent "$(git describe --tags --abbrev=0 HEAD^)"..HEAD ;;
esac
