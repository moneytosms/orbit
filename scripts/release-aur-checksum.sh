#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: release-aur-checksum.sh VERSION [--asset-dir DIR] [--repo-root DIR] [--dry-run]" >&2
  exit 2
}

version="${1:-}"
[[ -n "$version" ]] || usage
shift
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "release-aur: version must be X.Y.Z" >&2; exit 2; }

asset_dir=""
repo_root=""
dry_run=0
while (($#)); do
  case "$1" in
    --asset-dir) (($# >= 2)) || usage; asset_dir="$2"; shift 2 ;;
    --repo-root) (($# >= 2)) || usage; repo_root="$2"; shift 2 ;;
    --dry-run) dry_run=1; shift ;;
    *) usage ;;
  esac
done

if [[ -z "$repo_root" ]]; then
  repo_root="$(git rev-parse --show-toplevel)"
fi
aur_dir="$repo_root/packaging/aur"
pkgbuild="$aur_dir/PKGBUILD"
[[ -f "$pkgbuild" ]] || { echo "release-aur: missing $pkgbuild" >&2; exit 1; }
pkgver="$(awk -F= '$1 == "pkgver" { print $2; exit }' "$pkgbuild")"
[[ "$pkgver" == "$version" ]] || {
  echo "release-aur: PKGBUILD pkgver is $pkgver, expected $version; run just release first" >&2
  exit 1
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
if [[ -z "$asset_dir" ]]; then
  asset_dir="$tmp/download"
  mkdir -p "$asset_dir"
  gh release download "v$version" --repo moneytosms/orbit \
    --pattern "Orbit_${version}_amd64.AppImage" --dir "$asset_dir"
fi
image="$asset_dir/Orbit_${version}_amd64.AppImage"
[[ -s "$image" ]] || { echo "release-aur: missing staged AppImage: $image" >&2; exit 1; }
checksum="$(sha256sum "$image" | awk '{print $1}')"
[[ "$checksum" =~ ^[[:xdigit:]]{64}$ ]] || { echo "release-aur: failed to calculate AppImage checksum" >&2; exit 1; }

target="$aur_dir"
if ((dry_run)); then
  target="$tmp/aur"
  cp -a "$aur_dir" "$target"
fi
sed -i -E "s/^sha256sums=\('[^']*'\)$/sha256sums=('$checksum')/" "$target/PKGBUILD"
grep -Eq "^sha256sums=\('$checksum'\)$" "$target/PKGBUILD" || {
  echo "release-aur: could not update the PKGBUILD checksum" >&2
  exit 1
}
(cd "$target" && makepkg --printsrcinfo > .SRCINFO)
grep -q "sha256sums = $checksum" "$target/.SRCINFO" || {
  echo "release-aur: generated .SRCINFO checksum does not match" >&2
  exit 1
}

if ((dry_run)); then
  echo "would set orbit-bin $version AppImage checksum to $checksum and regenerate .SRCINFO"
else
  echo "updated $pkgbuild and $target/.SRCINFO for Orbit $version"
fi
