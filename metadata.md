# Project Metadata

## Identity

- Name: `mirror-komiku`
- Short Description: `Multi-version manga mirror reader with Rust server and static Edge proxy.`
- Topics: `manga reader rust actix-web edge-functions static-site indexeddb avif websocket`
- Usage Restriction: `Personal use only.`
- Liability Notice: `Provided as-is. Maintainer does not accept responsibility for any impact of use.`
- License: `Personal Use Only License (PUOL-1.0)` (see `LICENSE`)
- Repository: `https://github.com/ahrdadan/mirror-komiku`
- Live Static URL: `https://mirror-komiku.edgeone.dev/`

## Workspace Versions

- `versions/rust-server`
  - Stack: Rust + Actix Web
  - Purpose: server-side mirror/raw chapter pipeline
- `versions/static`
  - Stack: Static HTML/CSS/JS + Edge Function proxy
  - Purpose: client-driven reader with IndexedDB local cache

## CI/CD

- Workflow: `.github/workflows/ci.yml`
  - Rust checks:
    - `cargo fmt --manifest-path versions/rust-server/Cargo.toml --all -- --check`
    - `cargo check --manifest-path versions/rust-server/Cargo.toml --all-targets --all-features`
    - `cargo test --manifest-path versions/rust-server/Cargo.toml --all-targets --all-features -- --nocapture`
  - Static checks:
    - required file validation
    - JS syntax check for `versions/static/app.js`
    - JS syntax check for `versions/static/edge-functions/api/proxy.js`
- Workflow: `.github/workflows/rust-release.yml`
  - Reads app version from `versions/rust-server/Cargo.toml`
  - Builds Rust binary in release mode
  - Publishes GitHub Release with tag format `v<version>`

## Notes

- `versions/static/app.zip` is ignored via root `.gitignore`.
- Metadata workflow for GitHub sidebar is not used in this repository.
