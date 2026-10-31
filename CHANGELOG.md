# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-10-31

### Added
- **Hardware-Faithful UI Overhaul**: Upgraded widget to full 3D visor cowl hardware design matching high-fidelity hardware references and scroll-craft site. Features 72px lamp apertures (36px radius), 80px vertical step spacing, dark-matte capsule housing (`#191D23`), deep cylindrical hood visors with top rim highlights and inner dark lips, fine-grained 9x9 dotted LED matrix grid with active LED cores, smooth 250ms exponential color transition blending between states, and dual-stage ambient bloom halos.
- **Zero-Friction Automated State Engine**: Fully automated background transitions without manual approval prompts or terminal subshell spawning. The user only ever executes `/traffic on` (to launch/show) and `/traffic off` (to close/terminate).
- **Silent Background IPC Loopback**: State shifts across all AI coding agents (OpenCode, Qoder CLI, Antigravity/Gemini, Codex, Claude Code) stream silently over local loopback HTTP (`http://127.0.0.1:8765`), eliminating subshell noise (`npx -y`, `2>/dev/null`, piped shells) and CLI security warnings.

## [0.2.2] - 2026-10-30

### Fixed
- **Agent Lifecycle Event Transitions**: Fixed status event hooks so `traffic-status` actively updates colors during agent execution and permission prompts. Added listeners for `permission.updated`, `permission.ask`, `session.error` (🔴 Red), `chat.message`, `command.execute.before`, `tool.execute.before` (🟡 Yellow), and `session.idle`, `session.status: idle` (🟢 Green).
- **Tool Execution & CLI Fallbacks**: Added support for `traffic-status set <green|yellow|red> [MSG]` alongside direct state commands (`traffic-status <green|yellow|red> [MSG]`) for seamless integration across all AI coding agent skills and plugins.
- **Universal Agent Skill Guidelines**: Updated `/traffic` slash commands and skill instructions across OpenCode, Qoder, Codex, Antigravity, and Claude Code to enforce active lifecycle transitions.

## [0.2.1] - 2026-10-30

### Fixed
- **npm Platform Package Metadata**: Explicitly declared `"publishConfig": { "access": "public" }` in `@its-jani/traffic-status-win32-x64` manifest and `prepare.mjs` generator script to resolve npm OIDC trusted publisher provenance verification.
- **Repository URL**: Enforced canonical `git+https://github.com/its-jani/traffic-status.git` format across all npm package manifests.

## [0.2.0] - 2026-10-30

### Added
- **Authentic hardware UI redesign**: Polycarbonate capsule housing with beveled rim, cylindrical visor cowls (sun shades), concentric dotted LED emitter matrix, multi-stage ambient bloom halos, and hardware screws — matching real-world traffic light hardware aesthetics.
- **Multi-CLI agent support**: Slash commands (`/traffic on|off`) now work in Qoder CLI, Codex, Antigravity (AGY/Gemini), Claude Code, and OpenCode via per-project and global hook files.
- **Project-level BeautifulRust skills**: `rust-best-practices`, `rust-port`, and `rust-review` skills installed at project level (`.agents/skills/`, `.claude/skills/`, `.gemini/skills/`) for AI coding agent quality enforcement.

### Fixed
- **`/traffic off` now fully terminates the widget**: Previously only dismissed the current session. Now sends `POST /shutdown` to kill the daemon process entirely.
- **No taskbar icon**: Widget no longer appears in the Windows taskbar (`with_taskbar(false)`).
- **Default top-right placement**: Widget spawns at the top-right corner of the primary screen with a 28 px margin.

### Changed
- Window size tuned: 86×236 px (single session) or 118×236 px (multi-session).
- Yellow breathing pulse animation retained and refined.
- Hover-activated close/minimize controls (only visible on hover).

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
