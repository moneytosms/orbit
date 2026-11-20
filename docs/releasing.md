# Releasing Orbit

Releases are lockstep: one semver `X.Y.Z` applies to the Cargo workspace, the crate path-dep pins,
`app/package.json`, `app/src-tauri/tauri.conf.json` and `packaging/aur/PKGBUILD`.

## Cut a release

```sh
just release 0.1.0 --dry-run    # default: bumps a temp copy, verifies, prints the plan
just release 0.1.0 --publish    # bumps the repo, commits, tags v0.1.0, pushes
```

`--publish` needs a clean tree, a free tag and `git-cliff` on the PATH. The tag push starts
`.github/workflows/release.yml`, which builds and publishes:

- `ghcr.io/<owner>/<repo>:<version>` and `:latest` (linux/amd64, linux/arm64)
- `Orbit_<version>_amd64.AppImage` (Linux desktop)
- `Orbit-<version>-android.apk` (signed release APK)
- a GitHub Release `v<version>` with the changelog notes and all assets

The `release` recipe is provided by `release.just`; the justfile must contain `import 'release.just'`.

## Secrets (GitHub Actions)

| Secret | Used by |
| --- | --- |
| `GITHUB_TOKEN` (automatic) | GHCR push, `gh release create` |
| `ORBIT_KEYSTORE_B64` | base64 of the release keystore; decoded to a temp file |
| `ORBIT_KEYSTORE_PASSWORD` | keystore password |
| `ORBIT_KEY_ALIAS` | signing key alias |
| `ORBIT_KEY_PASSWORD` | signing key password |
| `TAURI_SIGNING_PRIVATE_KEY` | updater minisign private key; signs the AppImage (`~/.config/orbit/updater.key`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | updater key password |

The android job fails if `ORBIT_KEYSTORE_B64` is empty, so a release is never debug-signed.

Local signing (optional) uses `ORBIT_KEYSTORE`, `ORBIT_KEYSTORE_PASSWORD`, `ORBIT_KEY_ALIAS`,
`ORBIT_KEY_PASSWORD` env vars, or `~/.config/orbit/release.properties` with keys `storeFile`,
`storePassword`, `keyAlias`, `keyPassword`. Keep the keystore outside the repo. Without any of
these, the release APK is debug-signed.

APK versioning: `versionName` is the semver; `versionCode` is `major*10000 + minor*100 + patch`.

## Obtainium

Obtainium tracks the GitHub Release. Point it at the repo and keep the asset name pattern
`Orbit-.*-android\.apk`, so it picks the APK by name.

## Auto-updater

The Linux AppImage updates itself with `tauri-plugin-updater`. `tauri.conf.json` sets
`plugins.updater.pubkey` (the public half of the minisign key) and the endpoint
`https://github.com/moneytosms/orbit/releases/latest/download/latest.json`, and
`bundle.createUpdaterArtifacts` is `true`. The release workflow signs the AppImage and publishes
`Orbit_<version>_amd64.AppImage.sig` and `latest.json` next to it. The APK has no updater; Obtainium covers it.

The private key stays outside the repo at `~/.config/orbit/updater.key`. Never commit it. CI reads it from
the `TAURI_SIGNING_PRIVATE_KEY` secret, and its password from `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. If the key
is lost, installed copies cannot verify updates signed with a new key, so users would reinstall by hand.

## AUR (`orbit-bin`)

After the GitHub Release is published:

```sh
cd packaging/aur
updpkgsums                      # replaces SKIP with the real sha256 of the AppImage
makepkg --printsrcinfo > .SRCINFO
```

Push `PKGBUILD` and `.SRCINFO` to the `orbit-bin` AUR repository. See `packaging/aur/README.md`.

## Server image

```sh
docker run --rm -p 8080:8080 -v orbit-data:/data ghcr.io/<owner>/<repo>:<version>
```

The container runs as uid 10001, listens on `0.0.0.0:8080` and keeps its data in `/data`.
