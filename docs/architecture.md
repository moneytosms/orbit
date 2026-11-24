# Architecture tour

For contributors who want a map of the codebase before changing it: which crate owns what, and how a change travels from the UI to the server and back.

## Before you start

- Read [Decisions](adr/README.md), starting with [ADR 0001](adr/0001-stack.md), and skim the [Glossary](../GLOSSARY.md). Orbit's words (Area, Section, Phase, Check-in) have exact meanings.
- A checkout of the repository and the toolchain described in [Contributing](contributing.md).

## Steps

1. Know the shape. A Rust (axum) server with SQLite is the source of truth. Clients sync through client-generated ids, an idempotent command queue, sync-token deltas and per-field last-writer-wins. One Svelte 5 UI ships as a PWA and inside Tauri 2 (Linux and Android).

2. Find the crate that owns your change:

   | Path | Role |
   | --- | --- |
   | `crates/orbit-core` | shared logic: quick-add parser, recurrence, reminders, roadmap graph and code format, Stardust. Native and wasm; every function takes `now` and the time zone as input |
   | `crates/orbit-wasm` | wasm-bindgen wrapper over core, used by the UI |
   | `crates/orbit-client` | sync client, SQLite replica and outbox, used by the desktop app and the CLI |
   | `crates/orbit-server` | axum + SQLite server (`orbit-server`) |
   | `crates/orbit-cli` | the `orbit` CLI; `docs/cli.md` is generated from it |
   | `app/` | Svelte 5 + Vite UI (pnpm); `app/src-tauri` is the Tauri crate `orbit-app` |
   | `site/` | this guide and the landing page (Astro) |

3. Follow a write. The UI calls a function in `app/src/lib/sync.ts`, which builds a command, applies it to the local IndexedDB replica and queues it. Sync sends the queue to the server, which validates it with `orbit-core`, stores it with per-field clocks and answers with a delta. Other devices pull that delta. Because commands carry ids, a retried command is a no-op.

4. Put logic in core when two clients need it. Core is tested natively with `cargo test` and reaches the browser through `orbit-wasm`, so a parser or layout rule has one implementation.

5. Add a decision record when you choose between real options. Copy the shape of an existing file in `docs/adr/` (title, `Status:`, `Date:`, options, consequences) and add a row to the index in `docs/adr/README.md`.

## Verify it worked

From the repository root:

```sh
cargo test --workspace
```

All tests pass, and the crate you changed is covered by at least one. The full gate is in [Contributing](contributing.md).

## Troubleshooting

**The UI shows old behaviour after a core change.** The wasm build is stale. Run `just wasm` (the gate does it for you).

**A change syncs on one device but not another.** Check that the field has its own clock in core: per-field last-writer-wins only merges fields it knows about.

## Next

[Contributing](contributing.md): the gate, formatting and conventions.
