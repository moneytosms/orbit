# AUR: orbit-bin

`PKGBUILD` installs the Linux AppImage from the GitHub Release (`/usr/bin/orbit-app`). The AUR repo is separate from this one: copy `PKGBUILD` there and push it.

## Per release

After `just release X.Y.Z --publish` and the GitHub Release workflow finishes, run:

```sh
just release-aur X.Y.Z
```

This downloads the named AppImage with `gh`, computes its checksum, updates `PKGBUILD`, and
regenerates `.SRCINFO`. To stage or test a local asset without network access, pass its directory:

```sh
just release-aur X.Y.Z --asset-dir /path/to/dist --dry-run
```

The checked-in `PKGBUILD` keeps `SKIP` as a template until a release asset exists; do not copy that
placeholder to AUR. Copy the updated `PKGBUILD` and `.SRCINFO` into the AUR `orbit-bin` git repo,
then commit and push there. `makepkg --clean` remains an optional local package build check and needs
`webkit2gtk-4.1` installed.
