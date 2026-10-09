# CC Switch code origin

- Project: https://github.com/farion1231/cc-switch
- Pinned commit: `8596a233b2373226a0307d5f80ec0954d5fa162f`
- Upstream package.json version: 4.0.3
- License: MIT; the full statement is in `licenses/CC-Switch-MIT.txt`, which is also bundled inside the app package.

This project extracts CC Switch's configuration-modification engine, retains the React + Tauri stack, and rewrites it as a four-step connector.

| This project | Upstream path | Adjustments |
| --- | --- | --- |
| `src-tauri/src/patch/json.rs` | `src-tauri/src/live/patch/json.rs` | Keeps JSON patching and formatting, adjusts test module paths |
| `src-tauri/src/patch/toml.rs` | `src-tauri/src/live/patch/toml.rs` | Keeps TOML comment and structure patching, adjusts test module paths |
| `src-tauri/src/patch/mod.rs` | `src-tauri/src/live/patch/mod.rs` | Removes the database, whole-file engine, and AppError dependency, keeps the patch interface and parse errors |
| `src-tauri/src/provider_fields.rs` | `src-tauri/src/live/floor.rs` | Extracts the provider field definitions for Claude and Codex |
| `src-tauri/src/resources/codex_native_responses_template.json` | Same path | Copies the base template for the Responses catalog; name-only catalogs use conservative capability parameters, with gateway metadata taking priority |
| Claude model menu in `src-tauri/src/adapter.rs` | `src-tauri/src/mode/controller.rs` | References the upstream `modelPicker` custom-option structure and writes it according to the user's selections |
| `src-tauri/src/desktop_config.rs` | `src-tauri/src/claude_desktop_config.rs` | Adopts the 3P profile + metadata + deploymentMode layout; uses an independent profile ID and does not replicate the proxy service or the network-permission-widening settings |

The newly written code handles the single-page UI, model catalog fetching, client adaptation, backup transactions, and restore. Storage uses the same same-directory temporary-file atomic replacement strategy as upstream, implemented with tempfile, and does not copy upstream's SQLite, proxy, or account systems.

Upstream's proxy forwarding, automatic failover, statistics, MCP/Skills management, subscription login, cloud sync, tray, auto-update, and deep-link handlers are not included. No upstream update URL or app identifier is inherited, and the CC Switch database is not read.


Branding: the app UI and packaging icons use TokenToAPI's official brand (the T2A Emerald/Sky gradient mark); the brand assets come from the [TokenToAPI](https://tokentoapi.com) brand kit.

VS Code comment preservation uses `jsonc-parser` 0.34.0 (MIT), and its license is bundled with the package at `licenses/jsonc-parser-MIT.txt`.