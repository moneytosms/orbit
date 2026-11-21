#!/bin/sh
# Installs the `orbit` CLI from a GitHub release (Linux x86_64 and aarch64). No Rust needed.
#   curl -fsSL https://raw.githubusercontent.com/moneytosms/orbit/main/scripts/install-cli.sh | sh
# Env: ORBIT_VERSION=0.1.2 pins a version (default: latest), ORBIT_BIN=dir sets the install dir
# (default ~/.local/bin), ORBIT_RELEASES overrides the releases URL (for tests).
set -eu

releases="${ORBIT_RELEASES:-https://github.com/moneytosms/orbit/releases}"
bin="${ORBIT_BIN:-$HOME/.local/bin}"

[ "$(uname -s)" = Linux ] || { echo "orbit: this installer supports Linux only; build from source (see docs/cli-install.md)" >&2; exit 1; }
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  aarch64 | arm64) arch=aarch64 ;;
  *) echo "orbit: no prebuilt CLI for $(uname -m)" >&2; exit 1 ;;
esac

version="${ORBIT_VERSION:-}"
if [ -z "$version" ]; then
  # /releases/latest redirects to /releases/tag/vX.Y.Z
  version=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$releases/latest" | sed 's|.*/v||')
fi
[ -n "$version" ] || { echo "orbit: could not find the latest release" >&2; exit 1; }

name="orbit-$version-linux-$arch"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
base="$releases/download/v$version"

echo "Downloading orbit $version ($arch)"
curl -fsSL -o "$tmp/$name.tar.gz" "$base/$name.tar.gz"
curl -fsSL -o "$tmp/SHA256SUMS" "$base/SHA256SUMS"
(cd "$tmp" && grep " $name.tar.gz\$" SHA256SUMS | sha256sum -c -) || { echo "orbit: checksum mismatch, not installing" >&2; exit 1; }

tar -xzf "$tmp/$name.tar.gz" -C "$tmp"
mkdir -p "$bin"
install -m 755 "$tmp/$name/orbit" "$bin/orbit"
echo "Installed $bin/orbit"
case ":$PATH:" in *":$bin:"*) ;; *) echo "Add $bin to your PATH to run it." ;; esac
echo "Next: orbit login https://your-server"
