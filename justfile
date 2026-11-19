# fmt, clippy, cargo test, wasm build, svelte-check, vitest, web build
check:
    pnpm -C app install --frozen-lockfile
    pnpm -C app check
    pnpm -C app test
    pnpm -C app build
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo build -p orbit-wasm --target wasm32-unknown-unknown

# Run `just check` before every push
hooks:
    git config core.hooksPath .githooks
