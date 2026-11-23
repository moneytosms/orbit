#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: check-release-assets.sh VERSION DIST_DIR}"
dist="${2:?usage: check-release-assets.sh VERSION DIST_DIR}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "invalid version: $version" >&2; exit 2; }

required=(
  "Orbit_${version}_amd64.AppImage"
  "Orbit_${version}_amd64.AppImage.sig"
  "Orbit_${version}_x64-setup.exe"
  "Orbit_${version}_x64-setup.exe.sig"
  "latest.json"
  "Orbit-${version}-android.apk"
  "orbit-${version}-linux-x86_64.tar.gz"
  "orbit-${version}-linux-aarch64.tar.gz"
  "SHA256SUMS"
)
for name in "${required[@]}"; do
  [[ -s "$dist/$name" ]] || { echo "missing release asset: $name" >&2; exit 1; }
done
files=("$dist"/*)
[[ "${#files[@]}" -eq "${#required[@]}" ]] || {
  echo "staged release has unexpected assets" >&2
  exit 1
}
for arch in x86_64 aarch64; do
  tarball="orbit-$version-linux-$arch.tar.gz"
  tar -tzf "$dist/$tarball" | grep -Fx "orbit-$version-linux-$arch/orbit" >/dev/null || {
    echo "$tarball does not contain its named orbit executable" >&2
    exit 1
  }
done

jq -e --arg version "$version" --arg asset "Orbit_${version}_amd64.AppImage" \
  '.version == $version and (.platforms["linux-x86_64"].url | endswith($asset))' \
  "$dist/latest.json" >/dev/null || { echo "latest.json does not point to the versioned AppImage" >&2; exit 1; }
signature="$(jq -r '.platforms["linux-x86_64"].signature // empty' "$dist/latest.json")"
[[ "$signature" == "$(<"$dist/Orbit_${version}_amd64.AppImage.sig")" ]] || {
  echo "latest.json signature does not match the AppImage signature asset" >&2
  exit 1
}

jq -e --arg asset "Orbit_${version}_x64-setup.exe" \
  '.platforms["windows-x86_64"].url | endswith($asset)' \
  "$dist/latest.json" >/dev/null || { echo "latest.json does not point to the versioned Windows installer" >&2; exit 1; }
[[ "$(jq -r '.platforms["windows-x86_64"].signature // empty' "$dist/latest.json")" == "$(<"$dist/Orbit_${version}_x64-setup.exe.sig")" ]] || {
  echo "latest.json signature does not match the Windows installer signature asset" >&2
  exit 1
}

for name in "${required[@]}"; do
  [[ "$name" == SHA256SUMS ]] && continue
  (cd "$dist" && awk -v name="$name" '$2 == name && length($1) == 64 && $1 ~ /^[[:xdigit:]]+$/ { found = 1 } END { exit !found }' SHA256SUMS) || {
    echo "SHA256SUMS is missing $name" >&2
    exit 1
  }
done
(cd "$dist" && sha256sum -c SHA256SUMS --ignore-missing >/dev/null)
echo "release assets verified for $version"
