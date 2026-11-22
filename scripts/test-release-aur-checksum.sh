#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
version=1.2.3
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
repo="$tmp/repo"
dist="$tmp/dist"
mkdir -p "$repo/packaging" "$dist"
cp -a "$root/packaging/aur" "$repo/packaging/aur"
sed -i "s/^pkgver=.*/pkgver=$version/" "$repo/packaging/aur/PKGBUILD"
printf 'local AppImage fixture\n' > "$dist/Orbit_${version}_amd64.AppImage"

just --justfile "$root/justfile" release-aur "$version" \
  --asset-dir "$dist" --repo-root "$repo" --dry-run
grep -q "sha256sums=('SKIP')" "$repo/packaging/aur/PKGBUILD"
[[ ! -e "$repo/packaging/aur/.SRCINFO" ]]

just --justfile "$root/justfile" release-aur "$version" \
  --asset-dir "$dist" --repo-root "$repo"
expected="$(sha256sum "$dist/Orbit_${version}_amd64.AppImage" | awk '{print $1}')"
grep -q "sha256sums=('$expected')" "$repo/packaging/aur/PKGBUILD"
grep -q "sha256sums = $expected" "$repo/packaging/aur/.SRCINFO"
just --justfile "$root/justfile" release-aur "$version" \
  --asset-dir "$dist" --repo-root "$repo"
echo "AUR checksum fixture updated PKGBUILD and .SRCINFO without network"
