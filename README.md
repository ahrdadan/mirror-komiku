# mirror-komiku

[![CI](https://github.com/ahrdadan/mirror-komiku/actions/workflows/ci.yml/badge.svg)](https://github.com/ahrdadan/mirror-komiku/actions/workflows/ci.yml)

GitHub repository: <https://github.com/ahrdadan/mirror-komiku>

`mirror-komiku` is a multi-version workspace for manga chapter mirroring and reading, focused on sequential loading, prefetching, and cache efficiency.

## Workspace Layout

- `versions/rust-server`
  Rust + Actix Web server version (HTTP + WebSocket pipeline).
- `versions/static`
  Static frontend + EdgeOne Functions proxy version.

## Version Docs

- Rust server documentation: [`versions/rust-server/README.md`](versions/rust-server/README.md)
- Static version documentation: [`versions/static/README.md`](versions/static/README.md)

Live static deployment: <https://mirror-komiku.edgeone.dev/>

## GitHub CI/CD

This repository includes one GitHub Actions workflow:

- `CI` (`.github/workflows/ci.yml`)
  Runs checks for:
  - `versions/rust-server` (`cargo fmt`, `cargo check`, `cargo test`)
  - `versions/static` (required file checks + JavaScript syntax checks)

## Usage and Responsibility Notice

- This repository is provided for personal use only.
- The maintainer does not accept responsibility or liability for any direct or indirect impact from usage.
- You are fully responsible for your own usage, including legal and platform compliance.

## License

This project uses a custom restricted license:

- [LICENSE](LICENSE) - Personal Use Only License (PUOL-1.0)

## Notes

- Each version is intentionally independent.
- Changes in one version do not require shared module coupling with the other version.
