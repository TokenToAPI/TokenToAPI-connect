# Changelog

## v0.1.2 - 2026-10-12

- Made custom gateway settings continuously editable: adding a gateway selects it and keeps its settings open, while later edits remain in place with an explicit saved-state confirmation.
- Preserved the selected gateway across app restarts and prevented edits or deletions of other saved gateways from interrupting the active connection setup.
- Reject duplicate gateway names during preference recovery as well as interactive editing.

## v0.1.1 - 2026-10-11

- Fixed Codex reasoning-level catalogs for OpenAI-style and gateway-native model discovery responses, including gateway-provided capability metadata.
- Removed stale Codex and Claude compatibility switches during imports so they cannot force or disable reasoning after switching gateways.
- Moved restoration to the gateway list, with immediate restoration for a single unchanged backup and confirmation when local changes require review.
- Made successful imports clearer by showing the configured route, default model, imported model count, and client restart requirement.
- Restricted gateway model metadata to safe display and capability fields; gateway responses can no longer replace Codex instructions or tool settings.
- Corrected macOS compatibility documentation and package validation to macOS 12+ on Apple Silicon.
- Declared Node.js 22+ and pinned the development version to Node.js 22.23.3.

## v0.1.0 — 2026-10-08

First official public release.

- Four-step setup: choose a gateway, choose a client, enter your API Key, fetch and select models.
- TokenToAPI by default, with support for custom gateways and mutually exclusive single selection.
- Configures the Claude desktop app, Claude Code terminal / VS Code, and Codex desktop / terminal.
- Automatic backup, one-click restore, confirmation for later changes, and interrupted-operation recovery.
- Icon, colors, and light/dark themes that follow the site's branding.
- English user interface, error messages, and documentation.
- Ships a Windows x64 portable ZIP.
- MIT-licensed open source, retaining the CC Switch origin and third-party licenses.

See [VERIFICATION.md](VERIFICATION.md) for the build, tests, and known limitations.
