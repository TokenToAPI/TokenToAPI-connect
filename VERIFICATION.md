<!-- Website follow-up: see docs/APP_UPDATES.md. Stable aliases are optional;
the gateway discovers normal release packages automatically. No Connect app
binary or updater change is part of the Safari download fix. -->
# 2026-10-29 download and import follow-up — v0.2.1 unchanged

Stable direct-download aliases were added without rebuilding/replacing the
desktop packages or signed updater manifest. Both actual browser downloads
match original package digests. Validation passed: 99 Rust tests (including
combined handoff/import/restore), 19 helper/UI tests, 10 lifecycle tests,
TypeScript and seven real Codex runtime scenarios with loopback-only requests.
Native Windows install/upgrade/relaunch, settings preservation, Help menu and
stable-alias integrity passed in [run 40782429508](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/40782429508).
Gateway v0.2.19.3 is deployed; identical native effort animation remains
unresolved. Mac signing/notarization was excluded by the user. See
[the detailed evidence and limitations](docs/testing/connect-reliability-2026-10-29.md).

# v0.2.1 verification

Validated on 2026-10-22 on macOS arm64 using Node.js 24.13.0, pnpm 10.28.0, and Rust 1.99.1. Native Windows checks ran on GitHub's Windows runner.

- All 98 Rust unit/integration tests, 17 frontend tests, 10 lifecycle tests, TypeScript, production frontend build, Rust formatting, and desktop Clippy with warnings denied passed.
- Claude Code coverage verifies Tool Search preservation, explicit enable/disable, VS Code environment-variable behavior, and exact restoration. The UI documents Claude Code 2.1.14+, gateway forwarding of `tool_reference` blocks, and ToolSearch permissions; no automatic gateway capability claim is made.
- macOS arm64 and Windows x64 installer/portable packages were built with the existing updater signing key. Windows PE architecture, GUI subsystem, version, non-elevated execution, and static runtime dependency checks passed. Both ZIP integrity checks passed.
- Both updater signatures passed the actual Tauri verifier. An isolated copy of the v0.2.0 macOS app checked a loopback endpoint, downloaded the signed v0.2.1 archive, and installed it while preserving a configuration sentinel. Strict app-signature verification and version checks passed afterward. Corrupt bytes, substituted versions, and downgrades were rejected.
- The macOS ZIP and signed updater archive contain the same sealed app (all eight app files match).
- Native Windows v0.2.0 installation → v0.2.1 upgrade/relaunch, settings preservation, and Help-menu update dispatch passed: [run 39810330317](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/39810330317).
- All 24 platform handoff/download-resolver tests passed. The deployed `KeysView-Bx5tM5_q.js` uses GitHub's latest stable release API with caching disabled, prefers the Windows setup installer, and contains no pinned v0.1.3 download URL. No platform deployment is required for a new stable Connect release.
- After publication, the real GitHub updater endpoint offered v0.2.1 to v0.2.0, and the downloaded macOS updater passed signature verification. All nine public release downloads matched their local SHA-256 hashes.
- Executing the actual production `KeysView-Bx5tM5_q.js` download resolver selected the v0.2.1 macOS ZIP and Windows setup installer; both returned HTTP 200. The platform now follows the published release without a deployment.

No customer API keys or paid provider inference calls were used. A live Claude Code gateway conversation is not claimed. macOS remains ad-hoc signed/unnotarized; Windows is not publisher-signed. Missing MSVC debug-symbol files emitted linker warnings but did not prevent the optimized Windows build or native runtime checks.

# v0.2.0 verification

- 96 core Rust tests, 17 frontend tests and 10 lifecycle tests passed. New coverage checks native-menu events, busy/install guards, duplicate checks and listener cleanup.
- TypeScript, desktop Clippy with warnings denied, formatting and production builds passed.
- Light/dark footer appearance and client order were inspected. The actual packaged macOS app's **TokenToAPI Connect → Check for updates…** command displayed **You’re up to date.** in the footer.
- All seven native Codex 0.162.1 regression scenarios passed, including image-input capabilities and URL/local-image requests. Tests used isolated configuration and a loopback gateway.
- Actual v0.1.9 macOS bundle → network check → signed download → v0.2.0 installation passed, preserving a configuration sentinel. Both platform signatures passed; corrupt packages, substituted versions and downgrades were rejected.
- Native Windows v0.1.9 installation → v0.2.0 upgrade/relaunch, settings preservation and Help-menu update dispatch passed: [run 39437424439](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/39437424439).
- After publication, the real GitHub updater endpoint offered v0.2.0 to v0.1.9, and the downloaded macOS package passed signature verification. All nine published asset digests matched local files.
- The production Import to Connect resolver selected the v0.2.0 macOS ZIP and Windows setup installer, both HTTP 200. Live `KeysView-D9PCN1jz.js` contains the latest-release lookup and installer preference, with no pinned v0.1.3 download URL. No platform deployment was needed; its existing resolver follows the latest stable release.

No customer API keys were used. macOS remains ad-hoc signed/unnotarized and Windows is not publisher-signed. Native Windows UI feedback is covered by component tests; the Windows smoke test checks native menu dispatch and app survival.

# v0.1.9 verification

- 97 core Rust tests passed, including Grok import/reimport, unrelated settings preservation, exact restore, malformed TOML and missing-context rejection.
- TypeScript, 16 frontend tests and eight lifecycle tests passed. The lifecycle suite covers update checks, progress, installation/restart, errors and import locking.
- Official Grok Build 1.0.62 (1474f9cbdd91), on macOS, accepted the generated configuration, exposed only the selected models, sent authenticated Responses requests using the original model IDs, switched models with a supported reasoning effort, and restored the original configuration. Tests used a temporary home and fake loopback credentials.
- Native Codex 0.162.1 passed seven scenarios. The capability matrix covered 29 declarations, including four declared Grok vision models and one text-only Grok alias; 26 vision models sent 52 URL/local-file image turns. Grok declarations in this test are gateway fixtures, not a claim that every upstream Grok deployment supports images.
- Platform handoff tests passed for new and old clients, Responses routes, Claude-only restrictions and duplicate prevention. The latest-release download resolver passed all 24 tests; frontend locale checks and production build passed.
- Signed macOS update tests passed: actual v0.1.8 app copy → network check → download → v0.1.9 install, unchanged configuration sentinel, valid sealed app and correct version. Both package signatures passed; tampering, substituted versions and downgrades were rejected.
- Native Windows v0.1.8 install/launch → v0.1.9 upgrade/relaunch passed with settings preserved: [run 39429226752](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/39429226752).
- After publication, the real GitHub `latest.json` endpoint offered v0.1.9 to the v0.1.8 updater and its downloaded macOS package passed signature verification. All nine published asset digests matched the local release files.
- Production platform downloads resolved to v0.1.9 macOS ZIP and Windows setup installer, both HTTP 200. Public `KeysView-D9PCN1jz.js` contains the latest-release resolver and setup-installer preference, with no v0.1.3 download pin. Platform v0.2.17.3 deployment passed health/public-route checks.
- Desktop Clippy with warnings denied, formatting, overlay checks, integration audit and upstream preview passed. The old-release upgrade uses the same public signing key shipped in v0.1.8; no customer configuration was changed.

No customer API keys were used. Paid upstream inference and native Grok on Windows are not claimed by these tests. macOS remains ad-hoc signed/unnotarized and Windows is not publisher-signed.

# v0.1.8 verification

Release validation completed on 2026-10-20.

- 95 core Rust tests, TypeScript, 15 frontend tests and 8 UI lifecycle tests passed.
- The real signed macOS release passed verification and installation into an isolated temporary app bundle. Modified archive bytes and a substituted manifest version were rejected before replacement. A configuration/backup sentinel outside the bundle was preserved.
- This test caught AppleDouble metadata in the initial archive; packaging now excludes macOS extended attributes and resource-fork sidecars.
- macOS and Windows release packages built successfully. The updater signatures bind each package to version 0.1.8; the app requires this binding and rejects downgrades by default.
- Windows native installer smoke test passed on GitHub Actions: installation, native window launch, update-mode reinstallation, relaunch, and settings preservation. [Run 39420741989](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/39420741989).
- Both platform update signatures passed the real Tauri verifier. The release test also rejects downgrades. The macOS ZIP and update archive contain the same sealed app.
- Desktop Clippy with warnings denied and formatting passed. All seven native Codex 0.162.1 scenarios passed, including 24 capability declarations and 44 image turns across the 22 vision models.
- This is the first updater-enabled release. Windows testing exercised the actual installer in update mode; the UI download/install state flow is tested with mocked plugin events. No future-version production update is claimed.
- The signing private key is protected outside the repository. Neither release assets nor the Windows CI workflow contain it. The workflow requires contents-write solely to read draft releases, and does not publish or modify releases.
- OS publisher signing/notarization remains unchanged. Update signature verification is separate and mandatory.

# v0.1.7 verification

Release validation completed on 2026-10-20.

## Capability coverage

- Audited production account model mappings and active-key discovery without exporting credentials. The OpenAI groups expose the same four GPT models: GPT-5.5, GPT-5.6 Sol, GPT-5.6 Terra, and GPT-6 Astra. The live group-3 rich manifest advertises text/image input for all four; the plain list omits capability fields. Group 9 has the same allowlist but had no active key for a direct request. Claude-only group manifests remain text-only for native Codex and were not reclassified as image-capable Codex routes.
- Captured only safe capability fields in `docs/testing/production-gpt-catalog.json`. A regression verifies that all four live GPT entries retain image support after import. No backend change or restart is needed for this catalog, which already returns correct rich metadata.
- Verified 24 bundled capability declarations: 22 vision models and two text-only GPT-OSS models. Evidence comes from the installed Codex 0.162.1 native catalog and the provider registry already used by the gateway (`https://models.dev/api.json`). See `docs/testing/model-modality-evidence.json`. One legacy profile, `gpt-5-codex-mini`, had no verified modality record and therefore remains unverified rather than being guessed as vision-capable.
- Any model or alias can declare capabilities using `input_modalities`, `modalities.input`, `architecture.input_modalities`, or native `inputModalities`. Explicit flat gateway metadata has precedence. The native Codex cache fills missing capabilities only when neither gateway nor bundled metadata declares them. Unknown or malformed capabilities produce a named warning in the import summary.

## Automated checks

- 95 Rust tests passed, including all declared fallback modalities, the production catalog, arbitrary aliases in four metadata formats, gateway text-only precedence, native-cache fallback, and unknown-capability warnings.
- Desktop Clippy with warnings denied, formatting, TypeScript, 15 frontend tests, 3 lifecycle tests, and web production builds passed.
- Real Codex 0.162.1 passed seven isolated loopback scenarios. The new matrix checks all 24 declared models and sends both URL and local-file PNG input for each of the 22 vision models (44 image turns). Existing provider-authentication, context, compaction, stale-catalog repair, Astra, explicit vision, and restore checks also pass.
- Production image inference probes returned HTTP 403 `insufficient balance` before reaching the upstream. Consequently provider-side image acceptance has NOT been verified for the four live routes. No balances, customer settings, or service configuration were changed.

## Packages

- macOS arm64 built natively; version 0.1.7, macOS 12 minimum, protocol, architecture, strict ad-hoc signature, ZIP integrity, and SHA-256 passed.
- Windows x64 cross-compiled on macOS; PE/GUI architecture, non-elevated execution, static runtime dependencies, ZIP integrity, and SHA-256 passed. No native Windows launch is claimed. The linker emitted warnings about absent MSVC debug-symbol files and successfully produced the optimized binaries.
- Both packages remain unsigned by a publisher; macOS uses an ad-hoc signature and is not notarized.

```text
ef1650bc4ec5a67f6ef02d94c5bc8f1784bb52bfcb8afb35508ce3d5af38fda5  TokenToAPI-Connect-0.1.7-Windows-x64-portable.zip
8f5a07967d00517678946a0dd29def442f484d2428162cecac436eabb2ed78d4  TokenToAPI-Connect-0.1.7-macOS-arm64.zip
```

Install v0.1.7, fetch models again, click **Import**, fully quit/reopen Codex, and start a new chat. Updating Connect alone does not refresh an existing catalog. This release removes known discovery/import gaps; it does not promise that an arbitrary upstream accepts images or override an explicitly text-only route.

# v0.1.6 verification

Release validation completed on 2026-10-20.

## Automated checks

- 90 Rust unit/integration tests passed on macOS arm64, including ID-only vision discovery, case-insensitive profile matching, explicit gateway text-only precedence, and unsupported/duplicate modality filtering.
- Desktop Clippy with warnings denied, formatting, TypeScript, 15 frontend tests, 3 lifecycle tests, and the production web build passed.
- Real Codex `0.162.1` passed six isolated scenarios against a loopback-only fake gateway: provider credentials and restore, numeric thinking levels, legacy 32K repair, partial gateway metadata, explicit vision metadata, and ID-only Astra vision fallback.
- Both vision scenarios sent URL and local-file PNG inputs through Codex and verified `input_image` in the outgoing Responses requests. Reimport repaired a deliberately stale text-only catalog, subsequent reimport was stable, and the original configuration/authentication files restored exactly.
- The prior image harness entered its runtime block only for `runtime=True`, so the v0.1.5 vision case checked catalog import/restore without executing image turns. v0.1.6 corrects that guard, the catalog slug lookup, and model-specific context/reasoning expectations. Earlier image-runtime claims should be read with this correction.
- No real provider inference calls were made and no real user configuration was changed. The affected customer's installation has not been tested directly.

## Package validation

- macOS arm64 package built natively with app version `0.1.6`, macOS 12.0 minimum, registered import protocol, strict ad-hoc signature validation, and ZIP integrity checks.
- Windows x64 portable package cross-compiled on macOS with `cargo-xwin`; PE architecture, GUI subsystem, non-elevated execution, static runtime dependencies, and ZIP integrity checks passed. No native Windows launch/runtime test is claimed. MSVC's absent debug-symbol files emitted linker warnings; both optimized binaries linked successfully.
- Neither package is publisher-signed; the Mac app is ad-hoc signed and not notarized.

```text
30903249fcd9847623b3be910c18d09593f8c126ea8d40b36468aad086dad862  TokenToAPI-Connect-0.1.6-Windows-x64-portable.zip
5b30b619ae09edd584a16c4632a4e2b6079872df4ef2781a6da5d212c0768a3e  TokenToAPI-Connect-0.1.6-macOS-arm64.zip
```

Existing users must install v0.1.6, fetch their models again, click **Import**, then fully quit and reopen Codex. Updating Connect alone does not rewrite an existing model catalog. Explicit gateway capability limits remain authoritative.

No GitHub workflow is part of this release process.

# v0.1.5 verification

Reviewed the upstream SuperGPT Connect v0.1.6 compatibility release on 2026-10-19. The matching issues, fixes, and deliberately unsupported MCP Tool Search inference are documented in [COMPATIBILITY.md](COMPATIBILITY.md).

## Automated checks

- 87 Rust unit/integration tests passed on macOS arm64, including configuration, model discovery, backup/restore, Claude setup options, Codex catalog compatibility, and Windows path-resolution fixtures.
- TypeScript check, 15 frontend tests, 3 lifecycle tests, and the production web build passed on Node.js 26.11.0.
- Real Codex 0.162.0 passed five isolated macOS scenarios against a loopback-only fake gateway: provider credentials and restore, numeric thinking levels, legacy 32K catalog repair, partial gateway metadata, and URL/local image attachments.
- No real provider inference calls were made. The Claude Code binary installed on this Mac is 2.1.88, below the 2.1.242 custom-model-picker floor; Claude behavior is configuration-tested, not claimed as a live runtime validation.

## Package validation

- macOS arm64 package build passed with app version `0.1.5`, macOS 12.0 minimum system version, registered `tokentoapi-connect` protocol, and strict signature verification after extracting the release ZIP.
- Windows x64 portable package cross-compiled on macOS with `cargo-xwin`; PE architecture, GUI subsystem, non-elevated execution, static runtime dependencies, and ZIP integrity passed. This is not a native Windows launch or runtime test.
- Neither package is publisher-signed; the Mac app is ad-hoc signed and not notarized.

```text
02c5c9c0802576321a9901a1a32590d479b771ff21c3d51433c43f29a892a6d0  TokenToAPI-Connect-0.1.5-Windows-x64-portable.zip
efa0a80978275b62bb3c5f0c86b8bde2eba9c96349260773219a01e3db04773a  TokenToAPI-Connect-0.1.5-macOS-arm64.zip
```

No GitHub workflow is part of this release process.

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
