# AUR: orbit-bin

`PKGBUILD` installs the Linux AppImage from the GitHub Release (`/usr/bin/orbit-app`). The AUR repo is separate from this one: copy `PKGBUILD` there and push it.

## Per release

1. After `scripts/release.sh X.Y.Z --publish` and the release workflow finished, `pkgver` is already set. Fill in the checksum (downloads the AppImage; keep `sha256sums=('SKIP')` in this repo only until then):

   ```sh
   cd packaging/aur
   updpkgsums
   makepkg --printsrcinfo > .SRCINFO
   makepkg --clean   # optional: build check, needs webkit2gtk-4.1 installed
   ```

2. Copy `PKGBUILD` and `.SRCINFO` into the AUR `orbit-bin` git repo, commit, push.
