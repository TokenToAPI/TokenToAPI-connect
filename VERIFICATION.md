# v0.1.4 verification

Release validation completed on 2026-10-16.

## Codex compatibility

- 77 Rust tests passed, including 24 library tests and 6 Codex compatibility tests covering legacy 32K catalog repair, valid local context and auto-compaction overrides, gateway limit precedence, numeric thinking efforts, Ultra mapping, malformed metadata rejection, repeated import stability, and exact restore.
- The native Codex app-server regression used Codex `0.162.0` with a loopback-only fake gateway and temporary user homes. Four cases passed on macOS: profile/runtime, numeric effort, legacy 32K repair, and partial gateway repair.
- The regression verified the model list, selected request effort, effective context window, automatic compaction threshold, repeated import, and restoration without touching the real user's Codex files or making paid provider calls.
- The built-in profile table now covers the 22 models in the current native Codex catalog, including GPT-6.1 Sol / GPT-6 Astra Ultra and the GPT-5.x / GPT-6.x variants.

## macOS

- macOS arm64 package build passed with app version `0.1.4`, macOS 12.0 minimum system version, registered `tokentoapi-connect` protocol, and strict signature verification on the extracted release ZIP.
- SHA-256: `86ea537efda296c86b243e0b1eb4fc6e354e8ea78c98bef0108f766c4b5f3e82` for `TokenToAPI-Connect-0.1.4-macOS-arm64.zip`.

## Windows

- [Native Windows validation run](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/38850447155) passed on Windows Server 2022 x64 from the `0.1.4` release commit. It covers the x64 portable build, Rust/frontend checks, package integrity, launch, quoted per-user protocol registration, cold/running-app handoff, single-instance behavior, issuer review, and cancellation.
- The packaged-app smoke test uses a temporary runner profile and does not change real client configuration or make paid model requests.
- SHA-256: `2136ffcc9ccff3313e976771585a7f9e6176e88edd01cdade1e66ed168150ab8` for `TokenToAPI-Connect-0.1.4-Windows-x64-portable.zip`.

## Cross-platform Codex run

- [Codex compatibility workflow](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/38850742476) passed on both `macos-latest` and `windows-latest`, including Rust tests, the isolated importer fixture, Codex `0.162.0` app-server checks, and evidence upload.

## Previous v0.1.3 verification

### macOS

- Kendrick confirmed that both he and YC tested the deployed platform import successfully on their Macs. Individual OS/client versions were not supplied; see [the acceptance record](docs/testing/macos-acceptance.md).
- 67 Rust core tests, 12 frontend state tests and 3 lifecycle tests passed, along with TypeScript, build, formatting and Clippy.
- Isolated native Microsoft Edge checks covered cold/running-app handoff, gateway review, model fetching, fake-key import, cancellation and restore. Real client configuration hashes remained unchanged.
- Packaged arm64 executable, macOS 12 minimum, protocol registration, signature and ZIP integrity checks passed. The app remains ad-hoc signed and not notarized.
- The published Mac ZIP is unchanged from the accepted candidate. App source: `005b53a4f2c8decc79d4720c357109439d5dc0a7`, merged in `b48d8cd69aa7dcda31e7d270a56bbb51e8b94b3e`.

### Windows

- [Native Windows validation run](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/38276450134) passed on Windows Server 2022 x64, built from `76cd66a63234d138104890490478abadd4a5a410`.
- 12 frontend state tests, 3 lifecycle tests, TypeScript, 68 Rust tests (including Windows-specific paths), formatting, desktop Clippy and the Windows release build passed.
- The actual portable ZIP was extracted into a path with spaces and launched with WebView2. The smoke check verified quoted per-user protocol registration, cold and running-app handoff delivery, single-instance behavior, issuer review and cancellation.
- Rust fixture tests exercise configuration writes and restoration in temporary directories. The packaged-app smoke test does not change client configurations or perform real paid model requests. Real Claude/Codex application sessions on Windows 10/11 remain outside this automated check.
- PE32+ x64 GUI subsystem, v0.1.3 resources, non-elevated execution and ZIP integrity checks passed. No external VC++ or WebView2Loader DLL is required. WebView2 must be installed. The EXE is unsigned.
- The native Windows build uses the same app source as the accepted Mac build; intervening changes add packaging instructions, validation scripts and documentation.

### Published artifacts

Use the release's `SHA256SUMS.txt` to check both downloads:

```text
2136ffcc9ccff3313e976771585a7f9e6176e88edd01cdade1e66ed168150ab8  TokenToAPI-Connect-0.1.4-Windows-x64-portable.zip
86ea537efda296c86b243e0b1eb4fc6e354e8ea78c98bef0108f766c4b5f3e82  TokenToAPI-Connect-0.1.4-macOS-arm64.zip
```

The previous `v0.1.3` release checksums are retained below for historical reference:

```text
1a265818c5ce840be687f727146a1076004246643a8c3ee347851e931666ed76  TokenToAPI-Connect-0.1.3-Windows-x64-portable.zip
f4a54f8879cfde3a53cc46fc00f60e730c5d3c966af981b8922384675e85797d  TokenToAPI-Connect-0.1.3-macOS-arm64.zip
```

The platform provides both downloads through Import to Connect. Its frontend tests, type checking, lint, production build, custom overlay check and integration audit passed. GitHub platform CI could not start because of account billing limits; this is separate from the successful native Windows workflow.

Cleanup of temporary local build/test resources was explicitly authorized after publication and deployment. Preserve these release assets, credentials, installed apps and real client settings.

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

### Scope of verification

Missing WebView2, organization policy control, all client versions, and all combinations of model tool calls remain unverified item by item. Supporting a configuration format does not mean covering every historical client version, every gateway protocol, or every model capability.

The Windows release package is not yet signed. The Mac release package has not yet completed Apple Developer ID signing and notarization. The testing and packaging process does not modify real Claude / Codex configuration.
