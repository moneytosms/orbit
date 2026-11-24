# Contributing

For contributors who are about to open a change: how to set up, what the gate checks, and the conventions the repository expects.

## Before you start

- Rust (pinned in `rust-toolchain.toml`; rustup installs it and the wasm32 target on first build), `wasm-pack`, `just`, Node 24 with npm (the `site/` checks use npm), and `pnpm` 12. The Tauri crate is part of the workspace, so Linux also needs the WebKitGTK and tray libraries. Android work also needs a JDK 21 and the Android SDK (NDK 28). The maintainers keep everything under `~/.toolchain` with sccache; that is optional, use your own.

  Arch:

  ```sh
  sudo pacman -S --needed rustup just nodejs npm pnpm wasm-pack webkit2gtk-4.1 libayatana-appindicator librsvg
  rustup default stable
  ```

  Ubuntu (24.04; get Node 24 from NodeSource or nvm if `apt` has an older one):

  ```sh
  sudo apt install just libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  npm install -g pnpm@12
  cargo install wasm-pack
  ```

- Read the [Architecture tour](architecture.md).

## Steps

1. Install the git hook once per clone. It runs the gate before every push:

   ```sh
   just hooks
   ```

2. Make your change. Keep it to the task: match the surrounding style, no speculative options.

3. Format only the app files you touched, with the repository's Prettier config (single quotes, no semicolons, width 120):

   ```sh
   just fmt src/lib/sync.ts
   ```

4. Run the gate. It runs `pnpm install`, svelte-check, vitest, the web build, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, the wasm build and the site checks (build plus a docs lint for this guide):

   ```sh
   just check
   ```

5. Commit with a conventional message (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`), imperative, 72 characters or fewer, no trailing period. Use semantic lowercase branch names such as `feat/auth-refresh`.

6. Work from an issue. Issues live on GitHub; triage labels are `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` and `wontfix`. Docs for each convention are in `docs/agents/`.

## Verify it worked

`just check` exits with status 0 and prints `Static output links, images, anchors and screenshot placeholders look good.` at the end. The same command runs in CI (`.github/workflows/check.yml`).

## Troubleshooting

**The docs lint fails.** Every `docs/*.md` file must be listed in `site/guide.manifest.mjs` once, have one H1, give every fenced block a language, and follow the page shape. The message names the file and line.

**clippy fails on a warning in code you did not touch.** The gate uses `-D warnings`. Fix it, or note it in your change if it is pre-existing.

**`just check` is slow the first time.** Cargo and the web build populate caches; later runs are much faster.

## Next

[Releasing](releasing.md): how a maintainer cuts a release.
