# Connect v0.2.2 acceptance — 2026-11-03

Published [Connect v0.2.2](https://github.com/TokenToAPI/TokenToAPI-connect/releases/tag/v0.2.2) from reviewed merge `cff63bfd543be75e7513a6d687d3118cce92cdae` (PR #14). Packages add Gemini CLI native API setup, one default model, safe backup/restore, Gemini-aware handoff and distinct provider icons.

| Check | Result |
| --- | --- |
| Core Rust configuration tests | 107 on macOS; 108 on native Windows |
| Helper/UI and lifecycle tests | 21 and 11 passed |
| Static/build checks | TypeScript, strict desktop Clippy, Rust formatting and production builds passed |
| Real Gemini CLI 0.46.0 | Authenticated selected-model streaming with adapter-generated settings on macOS and Windows; loopback API only |
| Existing Codex compatibility | Seven real Codex 0.165.0 scenarios; 29 model declarations, 26 vision models and 52 image turns |
| Signed updater | Both signatures verified; tampering, substituted version and downgrades rejected |
| Mac upgrades | Three artifact/upgrade tests passed, including isolated actual v0.2.1 bundles, network check/download/install, strict sealed-app validation and preserved settings sentinel |
| Native Windows installer | Install/upgrade from v0.2.1, relaunch, preserved settings and Help-menu update dispatch passed |
| Public updater | Real endpoint offers v0.2.2 to v0.2.1; downloaded Mac archive passed the Tauri verifier |
| Published assets | All 12 downloaded public files match local accepted bytes |

Native Windows evidence: [run 41651389333](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/41651389333). The first run exposed an unescaped Windows path in an existing test fixture; JSON serialization corrected the fixture without changing the writer. An initial installer job began before draft upload finished; the complete-draft run passed. The Codex harness now terminates its own process group so temporary plugin-clone children cannot outlive cleanup.

The companion gateway v0.2.19.10 uses reviewed source `0db171deb281b02b0f0b0fc8d178387ddddf6c02`, preserving concurrent subscription UI work. Native Linux handoff/system-handler acceptance passed 67 test/subtest checks. Complete affected backend packages and vet passed. With a fresh frozen-lockfile install, all 3,220 frontend tests, TypeScript, locale validation and the production build passed. Three earlier frontend failures were caused by stale cached packages; they are superseded by the clean-install result. Gateway v0.2.19.9 was retired before this task installed it.

Public platform acceptance passed after gateway deployment at 21:37:22 Asia/Bangkok. `/api/v1/connect/download/mac` redirects to the normal v0.2.2 Apple Silicon ZIP; `/api/v1/connect/download/windows` redirects to the normal v0.2.2 Windows x64 setup installer. Both were downloaded through those platform routes and match the published package hashes. The signed updater and all 12 public assets also passed final verification. No paid provider inference, real customer handoff redemption or real client configuration writes were used. Browser preview covered desktop and minimum 380 × 620 widths; other browsers and devices were not tested in this release. macOS remains ad-hoc signed/unnotarized and Windows is not publisher-signed.
