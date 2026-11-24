# Contributing

For contributors who are about to open a change: how to set up, what the gate checks, and the conventions the repository expects.

## Before you start

- Rust (the version pinned in `rust-toolchain.toml`, with the wasm32 target), `pnpm`, `wasm-pack`, `just` and Node. Android work also needs the JDK and Android SDK. The maintainers keep these under `~/.toolchain`; you can use your own.
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
