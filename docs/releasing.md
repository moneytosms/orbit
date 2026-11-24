# Releasing Orbit

For maintainers cutting a release. Releases are lockstep: one semver `X.Y.Z` applies to the Cargo workspace, the crate path-dep pins,
`app/package.json`, and `app/src-tauri/tauri.conf.json`.

## Before you start

- Push access to the repository, a clean `main`, and the secrets listed under [Secrets](#secrets-github-actions) set in GitHub Actions.
- `just check` passes locally.

## Cut a release

```sh
just release 0.1.0 --dry-run    # default: bumps a temp copy, verifies, prints the plan
just release 0.1.0 --publish    # bumps the repo, commits, tags v0.1.0, pushes
```

`--publish` needs a clean tree, a free tag and `git-cliff` on the PATH. The tag push starts
`.github/workflows/release.yml`, which builds and publishes:

- `ghcr.io/<owner>/<repo>:<version>` and `:latest` (linux/amd64, linux/arm64)
- `Orbit_<version>_amd64.AppImage` (Linux desktop)
- `Orbit_<version>_amd64.AppImage.sig` and `latest.json` (desktop updater)
- `Orbit_<version>_x64-setup.exe` and `.exe.sig` (Windows installer, unsigned for SmartScreen; `.sig` feeds the updater)
- `Orbit-<version>-android.apk` (signed release APK)
- `orbit-<version>-linux-x86_64.tar.gz` and `orbit-<version>-linux-aarch64.tar.gz` (CLI)
- `SHA256SUMS` for the release assets
- a GitHub Release `v<version>` with the changelog notes and all assets

The `release` recipe is provided by `release.just`; the justfile must contain `import 'release.just'`.
Before creating the GitHub Release, the workflow verifies both GHCR tags and checks the staged
asset names, updater manifest, AppImage signature, APK, both CLI tarballs, and every `SHA256SUMS`
entry with `scripts/check-release-assets.sh`. Run it locally against a staged `dist/` directory
with `scripts/check-release-assets.sh X.Y.Z dist`; it performs no publishing.

## Client and protocol policy

- A patch or minor release without a protocol change is a server-only deploy. The server image
  carries the UI, so web, desktop and Android shells receive UI changes on reload; no native
  release is needed.
- A protocol bump changes `PROTOCOL_V` and `MIN_CLIENT_PROTOCOL`. Keep accepting protocol `N-1`
  for at least one minor release. Older clients receive HTTP 426 with an upgrade link and should
  preserve queued changes. CLI users can run `orbit self-update` to install a verified release.
- A native-only change, such as an Android notification or desktop tray fix, needs an APK/AppImage
  release. The UI should continue to work with older shells.

## Compatibility matrix

Sync protocol: the server is at `PROTOCOL_V = 1` and accepts clients down to `MIN_CLIENT_PROTOCOL`
(`PROTOCOL_V - 1`, never below 1). A client older than that gets HTTP 426 `upgrade_required` with the
upgrade link; a client newer than the server gets HTTP 400.

| Client | Ships as | Gets protocol changes by | Falls behind when | Fix |
| --- | --- | --- | --- | --- |
| Web / PWA | Served by the server (`ORBIT_WEB`) | Server deploy; the service worker updates itself | Never, it is the server's own UI | Reload the page |
| Linux desktop | AppImage (`Orbit_<version>_amd64.AppImage`) | Server deploy for UI; AppImage release for shell changes | Shell change needs a new AppImage | In-app updater |
| Android | APK (`Orbit-<version>-android.apk`) | Server deploy for UI; APK release for native changes | Native change needs a new APK | Install the new APK, or Obtainium |
| CLI | Linux tarballs (`orbit-<version>-linux-<arch>.tar.gz`) | Its own release | Gets HTTP 426 after a protocol bump | `orbit self-update` |

Protocol bumps: keep `N-1` for at least one minor release, so only clients two protocols behind see 426.

## Operator update path

Most client changes ship with the server image. Pull the desired version and restart the container;
the served UI updates with that deploy. Use a native release only for shell or protocol changes.
`orbit status` reports the server version/protocol and whether the CLI needs updating.

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

## Server image

```sh
docker run --rm -p 8080:8080 -v orbit-data:/data ghcr.io/<owner>/<repo>:<version>
```

The container runs as uid 10001, listens on `0.0.0.0:8080` and keeps its data in `/data`.

## Verify it worked

The release workflow finishes green and the GitHub Release lists the AppImage with its signature, the Windows installer, `Orbit-<version>-android.apk`, the CLI archives and `latest.json`. Then check the update path:

```sh
orbit self-update --dry-run
```

It downloads and verifies the new CLI archive without replacing the executable.

## Troubleshooting

**A release job fails on signing.** The signing secrets are missing or wrong. Re-check [Secrets](#secrets-github-actions); the Android key and the Tauri updater key are separate.

## Next

[Deploy Orbit](deploy.md): how operators pick up the new server image.
