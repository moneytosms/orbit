#!/usr/bin/env bash
set -euo pipefail

dist="${1:?usage: write-release-checksums.sh DIST_DIR}"
[[ -d "$dist" ]] || { echo "release directory not found: $dist" >&2; exit 1; }

# Remove prior output before expanding the asset list so repeated runs never hash SHA256SUMS.
rm -f "$dist/SHA256SUMS"
assets=()
for path in "$dist"/*; do
  [[ -f "$path" ]] && assets+=("${path##*/}")
done
((${#assets[@]} > 0)) || { echo "no release assets in $dist" >&2; exit 1; }
(cd "$dist" && sha256sum -- "${assets[@]}" > SHA256SUMS)
