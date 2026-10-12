# v0.1.3 release candidate verification

Prepared on 2026-10-12. This is a local candidate, not a published release.

- 67 Rust core tests pass, including strict handoff parsing, replay-sensitive redemption fixtures, error redaction, native credential retention and cancellation.
- 12 frontend state tests and 3 App/RestorePanel lifecycle tests pass. TypeScript, frontend build, desktop Rust compilation and Clippy pass.
- Home-page cards now use a consistent 10px gap; verified in Microsoft Edge and the native window.
- A separately identified debug app uses a temporary home and fake fixture key for browser import/restore checks. Microsoft Edge app-opening, gateway review, model fetching, cold/running-app receipt, import, cancellation and restore are verified in that isolated environment. Three real configuration file hashes stayed unchanged.
- Regression checks cover mixed-group client choice, Claude Code-only restrictions, queued imports during restore, normalized unsupported-group errors, and configured-to-empty gateway changes.
- The macOS arm64 candidate is built locally. Signature, version, protocol registration and archive integrity are checked before handover. It remains ad-hoc signed and not Developer ID notarized.
- Windows runtime and portable protocol registration have not been executed on Windows for this candidate. Its package remains a release-preparation step; do not imply that the macOS test validates Windows.
- Platform frontend tests pass (2,687); full backend unit and Docker-backed PostgreSQL/Redis integration suites pass after stale password fixtures were corrected without changing production policy. The previous four account-test failures and five more found by broader coverage are resolved.

Candidate SHA-256: `118966045b8c903d555debe5ce2db35ce642f9ea1cb770faa515cf1e44076740`. This package was built from the local working tree, including uncommitted spacing edits; rebuild and regenerate the checksum from the final committed source before publication.

The [GitHub release preparation notes](docs/releases/v0.1.3-macos.md) and [Kendrick/YC acceptance checklist](docs/testing/macos-acceptance.md) are ready. Both live Mac results remain Pending; Windows follows those results.

For the isolated handoff fixture, run scripts/mock-gateway.py and open
http://127.0.0.1:18473/fixture/launch. Loopback HTTP handoffs work only in an
isolated debug build; release builds require HTTPS. Never use real keys for
these tests or upload restore records.

## Previous v0.1.2 verification record


Verification date: 2026-10-12 (v0.1.2). The public v0.1.1 release was previously verified on 2026-10-11.

> **TokenToAPI Connect note (2026-10-07)**: this project's configuration engine, client adaptation, and test logic
> originate from CC Switch's open-source configuration engine (MIT licensed, see [UPSTREAM.md](UPSTREAM.md)), and it has fully
> adopted the TokenToAPI brand — the default gateway is [https://tokentoapi.com](https://tokentoapi.com)
> (a `/v1` gateway); the app icon, name, identifier, storage directory, and colors all use TokenToAPI's official brand
> (T2A Emerald / Sky). The models available from the default site are whatever the TokenToAPI gateway's `/v1/models` actually returns.

## Automated tests

- macOS / Apple Silicon: 61 Rust configuration tests and 10 frontend state tests pass; the TypeScript check, frontend production build, Rust formatting check, and Clippy pass.
- Configuration tests cover preserving the original restore point after multiple imports, exact restoration of an existing authentication file, reverting a newly created file, comment preservation, rejecting a stale preview, confirming later edits, interrupted-operation recovery, corrupted configuration, file locks, and paths containing non-ASCII characters and spaces.
- Model discovery tests use a loopback address to mock the gateway with fake keys, checking authentication, redirect rejection, model selection, and binding of the fetch results, without calling any real inference endpoint.
- The Windows-specific configuration test code includes `.cmd` launcher and AppData path checks and has been cross-compiled. The automated tests above were run on macOS.

## Artifact checks

- The macOS UI passed isolated import, restore, light/dark theme, and small-window checks. The public release unifies the version identity and the packaging method.
- Windows x64 EXE checks cover the PE architecture, GUI subsystem, 0.1.2 version resources, app icon, `asInvoker` permissions, and system DLL imports. No external VC++ or WebView2Loader DLL is required; the actual UI still needs the system WebView2 Runtime.
- The Mac package is checked for arm64, the 0.1.2 app version, macOS 12 minimum system version, and the ad-hoc signature; the ZIPs for both platforms are checked for CRC, archiver, and build artifact consistency, and SHA-256 is provided.

## Scope of verification

Missing WebView2, organization policy control, all client versions, and all combinations of model tool calls remain unverified item by item. Supporting a configuration format does not mean covering every historical client version, every gateway protocol, or every model capability.

The Windows release package is not yet signed. The Mac release package has not yet completed Apple Developer ID signing and notarization. The testing and packaging process does not modify real Claude / Codex configuration.
