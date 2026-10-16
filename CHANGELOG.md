# Changelog

## v0.1.4 - 2026-10-16

- Fixed Codex imports that could leave the effective context window at the legacy 32K fallback.
- Uses the native Codex model profiles for context capacity, compaction thresholds, thinking levels, numeric efforts, and Ultra mapping instead of guessing from a model name.
- Preserves valid user-edited `model_context_window`, `model_auto_compact_token_limit`, and supported thinking preferences across repeated imports; gateway-provided limits remain authoritative.
- Repairs old generated catalogs while rejecting missing or malformed capability metadata before any client file is changed.
- Added native Codex app-server regression coverage for model selection, context capacity, automatic compaction, repeated import, restoration, numeric efforts, and cross-platform fixtures.

## v0.1.3 - 2026-10-12

- Added browser-to-app platform imports through the tokentoapi-connect protocol.
- Redeems short-lived, single-use handoff codes in native memory; keys stay out of app-opening URLs and browser storage.
- Prefills gateway and client, fetches models, and requires Import before changing configuration.
- Supports cold and running-app handoffs, unfamiliar-platform review, retry, and cancellation.
- Keeps the imported gateway name in the success summary and preserves existing backup/restore behavior.
- Enforces authoritative allowed clients for mixed and Claude Code-only groups; queues imports while restore review is open.
- Adds consistent spacing between the home-page gateway, restore and add-gateway cards.
- Adds per-user Windows protocol registration for the portable app; open it once after extracting or moving its folder.
- Validates Windows packaging and native handoff behavior on a Windows runner after reported Mac acceptance.


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
