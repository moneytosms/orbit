# fmt, clippy, cargo test, wasm build, svelte-check, vitest, web build
check: wasm
    pnpm -C app install --frozen-lockfile
    pnpm -C app check
    pnpm -C app test
    pnpm -C app build
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    just demo
    npm --prefix site ci
    npm --prefix site run check
    npm --prefix site run build
    npm --prefix site run verify

# Build the in-browser web demo into site/public/demo (served at /demo/; needs `just wasm` first)
demo:
    VITE_DEMO=1 VITE_DEMO_BASE=/orbit/demo/ pnpm -C app build
    mkdir -p site/public/demo
    cp -r app/dist-demo/. site/public/demo/

# Format the given app files (paths relative to app/), e.g. `just fmt src/lib/sync.ts`; plus cargo fmt
fmt *files:
    cargo fmt --all
    {{ if files != "" { "pnpm -C app format " + files } else { "" } }}

# Build orbit-core for the browser into app/wasm
wasm:
    wasm-pack build crates/orbit-wasm --target web --out-dir ../../app/wasm --no-pack

# Sync server on 127.0.0.1:8080, data in ./data
server:
    cargo run -p orbit-server

# Web app dev server; proxies /sync to `just server`
web: wasm
    pnpm -C app install
    pnpm -C app dev

# Regenerate docs/cli.md and the man pages in docs/man from the CLI definitions
docs:
    cargo run -q -p orbit-cli -- docs docs

import 'release.just'

# Run `just check` before every push
hooks:
    git config core.hooksPath .githooks
