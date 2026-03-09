# mirror-komiku

[![CI](https://github.com/ahrdadan/mirror-komiku/actions/workflows/ci.yml/badge.svg)](https://github.com/ahrdadan/mirror-komiku/actions/workflows/ci.yml)
[![Repository Metadata](https://github.com/ahrdadan/mirror-komiku/actions/workflows/repository-metadata.yml/badge.svg)](https://github.com/ahrdadan/mirror-komiku/actions/workflows/repository-metadata.yml)

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

This repository includes two GitHub Actions workflows:

- `CI` (`.github/workflows/ci.yml`)
  Runs checks for:
  - `versions/rust-server` (`cargo fmt`, `cargo check`, `cargo test`)
  - `versions/static` (required file checks + JavaScript syntax checks)
- `Repository Metadata` (`.github/workflows/repository-metadata.yml`)
  Syncs repository sidebar metadata from `.github/repository-metadata.json`:
  - Description
  - Homepage URL
  - Topics (tags)

## Sidebar Metadata Setup

For reliable metadata updates, create this repository secret:

- `REPO_ADMIN_TOKEN`
  A GitHub Personal Access Token (PAT) with repository administration permission (`repo` scope for classic PAT, or `Administration: Read and write` for fine-grained PAT on this repo).

The metadata source file is:

- `.github/repository-metadata.json`

You can trigger metadata sync by:

- pushing changes to `.github/repository-metadata.json`, or
- running the `Repository Metadata` workflow manually from Actions.

## Notes

- Each version is intentionally independent.
- Changes in one version do not require shared module coupling with the other version.
