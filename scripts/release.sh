#!/usr/bin/env bash
# Lockstep release: scripts/release.sh X.Y.Z [--dry-run|--publish]
#
# Default is a dry run: the versions are bumped in a temp copy of the working tree and
# verified there; nothing in the repo is changed and nothing is committed, tagged, pushed
# or published. --publish bumps the repo, commits, tags vX.Y.Z and pushes. The tag push
# starts .github/workflows/release.yml, which builds the GHCR image, AppImage, APK and CLI
# assets, verifies their names and checksums, then creates the GitHub Release.
set -euo pipefail

usage() {
  echo "usage: scripts/release.sh X.Y.Z [--dry-run|--publish]" >&2
  exit 2
}

VERSION="${1:-}"
[[ -n "$VERSION" ]] || usage
shift
PUBLISH=0
for arg in "$@"; do
  case "$arg" in
    --publish) PUBLISH=1 ;;
    --dry-run) PUBLISH=0 ;;
    *) usage ;;
  esac
done
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "release: version must be X.Y.Z, got '$VERSION'" >&2
  exit 2
}
TAG="v$VERSION"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

step() { echo "==> $*"; }
# Runs the command for real with --publish; otherwise prints it.
run() {
  if ((PUBLISH)); then
    "$@"
  else
    echo "    would run: $*"
  fi
}

if ((PUBLISH)); then
  step "preflight (publish)"
  [[ -z "$(git status --porcelain --untracked-files=no)" ]] || {
    echo "release: working tree has uncommitted changes" >&2
    exit 1
  }
  git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && {
    echo "release: tag $TAG already exists" >&2
    exit 1
  }
  command -v git-cliff >/dev/null || {
    echo "release: git-cliff is required for the changelog: https://git-cliff.org/docs/installation" >&2
    exit 1
  }
  WORK="$ROOT"
else
  step "dry run: copying the working tree to a temp dir (repo untouched)"
  WORK="$(mktemp -d)"
  trap 'rm -rf "$WORK"' EXIT
  git ls-files -co --exclude-standard -z | while IFS= read -r -d '' f; do
    if [[ -e "$f" ]]; then cp --parents "$f" "$WORK/"; fi
  done
  cd "$WORK"
fi

step "bump lockstep version to $VERSION in $WORK"
sed -i -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"$/version = \"$VERSION\"/" Cargo.toml
sed -i -E "s/(orbit-[a-z]+ = \{ version = \")[0-9.]+(\", path)/\1$VERSION\2/" crates/*/Cargo.toml
sed -i -E "s/^  \"version\": \"[^\"]+\"/  \"version\": \"$VERSION\"/" app/package.json app/src-tauri/tauri.conf.json
sed -i -E "s/^pkgver=.*/pkgver=$VERSION/" packaging/aur/PKGBUILD

step "verify lockstep (cargo resolves the workspace, updates Cargo.lock)"
cargo metadata --format-version 1 >/dev/null
for f in Cargo.toml app/package.json app/src-tauri/tauri.conf.json packaging/aur/PKGBUILD; do
  grep -q "$VERSION" "$f" || {
    echo "release: $f did not take version $VERSION" >&2
    exit 1
  }
done
(cd packaging/aur && makepkg --printsrcinfo >/dev/null)
echo "    versions: $(grep -h '^version = ' Cargo.toml) / package.json $(grep -m1 '"version"' app/package.json | tr -d ' ,') / tauri.conf.json $(grep -m1 '"version"' app/src-tauri/tauri.conf.json | tr -d ' ,')"

step "changelog (git-cliff, conventional commits)"
if command -v git-cliff >/dev/null; then
  (cd "$ROOT" && git-cliff --tag "$TAG" -o "$WORK/CHANGELOG.md")
  if ((PUBLISH)); then
    echo "    wrote CHANGELOG.md"
  else
    echo "    top of CHANGELOG.md (dry run, not kept):"
    head -n 12 "$WORK/CHANGELOG.md" | sed 's/^/      /'
  fi
elif ((PUBLISH)); then
  exit 1
else
  echo "    skipped: git-cliff not installed (https://git-cliff.org/docs/installation); publish needs it"
fi

if ((PUBLISH)); then
  step "commit and tag"
  cd "$ROOT"
  git add Cargo.toml Cargo.lock crates/*/Cargo.toml app/package.json app/src-tauri/tauri.conf.json \
    packaging/aur/PKGBUILD CHANGELOG.md
  git commit -m "chore: release $TAG"
  git tag -a "$TAG" -m "Orbit $TAG"
fi

step "publish"
run git push origin HEAD
run git push origin "$TAG"
echo "    tag push starts .github/workflows/release.yml: GHCR image, signed AppImage/APK, CLI tarballs,"
echo "    checksummed GitHub Release assets."
echo "    After the release workflow finishes, run: just release-aur $VERSION"
echo "    Then copy packaging/aur/PKGBUILD and .SRCINFO to orbit-bin and push to the AUR."

if ((PUBLISH)); then
  echo "released $TAG"
else
  echo "dry run complete: nothing committed, tagged, pushed or published"
fi
