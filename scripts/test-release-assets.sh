#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
version=1.2.3
dist="$(mktemp -d)"
trap 'rm -rf "$dist"' EXIT
names=(
  "Orbit_${version}_amd64.AppImage"
  "Orbit_${version}_amd64.AppImage.sig"
  "Orbit_${version}_x64-setup.exe"
  "Orbit_${version}_x64-setup.exe.sig"
  "Orbit-${version}-android.apk"
  "orbit-${version}-linux-x86_64.tar.gz"
  "orbit-${version}-linux-aarch64.tar.gz"
)
for name in "${names[@]}"; do printf '%s fixture\n' "$name" > "$dist/$name"; done
for arch in x86_64 aarch64; do
  folder="orbit-$version-linux-$arch"
  mkdir -p "$dist/$folder"
  printf 'fixture CLI\n' > "$dist/$folder/orbit"
  tar -czf "$dist/$folder.tar.gz" -C "$dist" "$folder"
  rm -rf "$dist/$folder"
done
signature="$(<"$dist/Orbit_${version}_amd64.AppImage.sig")"
win_signature="$(<"$dist/Orbit_${version}_x64-setup.exe.sig")"
jq -n --arg version "$version" --arg signature "$signature" --arg win_signature "$win_signature" \
  --arg url "https://example.test/Orbit_${version}_amd64.AppImage" \
  --arg win_url "https://example.test/Orbit_${version}_x64-setup.exe" \
  '{version:$version,platforms:{"linux-x86_64":{signature:$signature,url:$url},"windows-x86_64":{signature:$win_signature,url:$win_url}}}' > "$dist/latest.json"
"$root/scripts/write-release-checksums.sh" "$dist"
"$root/scripts/check-release-assets.sh" "$version" "$dist"
"$root/scripts/write-release-checksums.sh" "$dist"
if grep -q 'SHA256SUMS' "$dist/SHA256SUMS"; then
  echo "checksum generator included its own output" >&2
  exit 1
fi
"$root/scripts/check-release-assets.sh" "$version" "$dist"

printf 'corrupt\n' >> "$dist/Orbit-${version}-android.apk"
if "$root/scripts/check-release-assets.sh" "$version" "$dist" >/dev/null 2>&1; then
  echo "release asset check accepted a corrupted APK" >&2
  exit 1
fi
echo "release asset fixtures rejected corrupted content"
