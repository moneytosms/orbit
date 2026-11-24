# Release recipes; imported by the justfile. Dry run unless --publish, e.g. `just release 0.1.0 --dry-run`.

# Bump the lockstep version to X.Y.Z and tag it (see scripts/release.sh)
release *args:
    scripts/release.sh {{ args }}
