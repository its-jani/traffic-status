# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-10-17

### Fixed
- `SHA256SUMS` was published without filenames because `Get-FileHash` returns objects exposing `Path` and `Hash` but no `Name` property, so `$_.Name` expanded to an empty string. `install.ps1` matches its archive by filename and therefore could never find a checksum, making every standalone (Node-free) installation fail. The filename is now derived from `Path` via `Split-Path -Leaf`.
- npm metadata: `repository.url` normalized to the `git+https://` form in both the launcher package and the `prepare.mjs` platform-package generator, so `npm publish` no longer auto-corrects the manifest (and the correction was previously reverted on every `prepare.mjs` run).
- npm metadata: the launcher package now ships the `LICENSE` file it already declared in `files`, declares `publishConfig.access`, and uses the canonical `bin` path without a `./` prefix.

## [0.1.0] - 2026-10-07

### Added
- Renamed project to `traffic-status` with clean single-command distribution.
- Zero-dependency Node.js distribution via `npm i -g traffic-status` and `npx traffic-status install --global`.
- Node-free standalone installer scripts `install.ps1` (Windows) and `install.sh` (Linux/macOS) with SHA-256 verification.
- Safe installation and uninstallation with marker headers (`<!-- generated-by: traffic-status -->`), protecting user configuration files from destruction.
- Subcommand `traffic-status doctor` for diagnosing PATH, background daemon status, and active integrations.
- Hardened loopback REST API server:
  - Binds to `127.0.0.1` only.
  - Strict method check (POST required for state mutations).
  - Strict `Content-Type: application/json` enforcement.
  - 64KB request body size limit.
  - Rejection of cross-origin state mutations (CSRF mitigation).
  - Port customization via `TRAFFIC_STATUS_PORT`.
- Migration and cleanup logic for previous `traffic-light` installations.
- Automated GitHub Actions CI and Release pipelines.
