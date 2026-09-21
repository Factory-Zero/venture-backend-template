# Local development
dev:
    wrangler dev

# Full local gate: formatting, lints, tests, doctor, and a real worker build.
# Stops at the first failure (just's default).
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo run --bin fz -- doctor
    worker-build --release

# Apply D1 migrations to the staging environment's remote database.
migrate-staging:
    wrangler d1 migrations apply DB --env staging --remote

# Apply D1 migrations to the production environment's remote database.
migrate-prod:
    wrangler d1 migrations apply DB --env production --remote
