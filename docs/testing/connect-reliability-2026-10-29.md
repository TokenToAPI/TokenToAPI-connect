# Connect download and import follow-up — 2026-10-29

## Scope and results

The user reported intermittent download buttons opening GitHub's release page,
Safari automatically expanding ZIPs, unavailable macOS security controls, and
an effort-slider animation difference in ChatGPT desktop on Mac and Windows.
The user explicitly excluded Developer ID signing/notarization and the disabled
Open Anyway problem from this follow-up.

The old website initialized download anchors to the release page and retained
that fallback after API errors or a five-second timeout. Replace that browser
lookup with GitHub's documented `/releases/latest/download/<asset-name>` URLs.
Stable Mac ZIP and Windows installer aliases are exact copies of v0.2.1's
published packages; no app executable, updater manifest or existing asset was
replaced. Both public URLs were downloaded and their SHA-256 values matched the
original GitHub asset digests.

`scripts/prepare-download-aliases.mjs` creates both aliases and their separate
checksum file. Manifest preparation calls it for future releases and includes
the aliases in checksums. Publish both stable names before making a release
Latest; missing/empty input fails before changing either alias. Keep versioned
updater URLs/signatures unchanged. The website no longer depends on a client
GitHub API request, page-load timing or the API rate limit.

## Import and effort validation

- 99 Rust tests passed, including a new combined loopback handoff redemption →
  Codex/Codex Desktop import → supported Ultra effort preservation → exact
  original-config restoration test. Only temporary homes and fake keys used.
- The real installed Codex CLI 0.165.0-alpha.1.2 passed all seven existing
  runtime scenarios using Connect's production importer and a loopback gateway:
  supported model/effort discovery, named and numeric effort requests, stale
  context repair, image input and the full capability matrix. No paid provider
  calls or real client configurations were used.
- Frontend helper/UI tests, lifecycle tests and TypeScript passed. Release-alias
  regressions cover exact bytes/checksums, successive versions and missing/empty
  inputs. This is test and release-tooling work; Connect stays at v0.2.1.

## Native animation boundary

Read-only inspection of ChatGPT desktop 26.1023.51421 on this Mac finds the
animated power picker in the desktop application's own UI. It receives simple/
advanced view, power selections and reduced-motion state. The local API model
picker consumes native `supportedReasoningEfforts`; correct effort discovery
and request behavior is verified above. There is no verified Connect catalog
field that enables the official ChatGPT power-picker animation for custom API
models. Changing a model ID or inventing capabilities is not a valid fix.

The requested identical animation on both operating systems remains unresolved
and requires support in the desktop client. No installed application bundle,
user motion preference, authentication state or model identity was modified.
The Windows visual behavior was not newly tested on a physical Windows device.

## Windows and gateway acceptance

Native Windows installation, upgrade from v0.2.0, settings preservation, real
window launch/relaunch and Help-menu update dispatch passed in
[run 40782429508](https://github.com/TokenToAPI/TokenToAPI-connect/actions/runs/40782429508).
The workflow selects versioned installers explicitly and verifies stable-alias
SHA-256 equality before installation. Current v0.2.1 updater manifest and
package digests are unchanged after adding aliases.

The matching website fix is deployed as gateway v0.2.19.3 from reviewed source
`e3da1c173466bb9cdd801f3846556db5c61ef480` at 05:47 Asia/Bangkok on 2026-10-29.
Both real browser downloads completed with matching package digests. Phone
width 390px fits both menu links without page overflow. Managed Linux tests,
live health/public paths and public build-asset comparisons passed.

## Safari installation notes

Safari can automatically decompress downloaded ZIPs. Use the extracted folder's
TokenToAPI Connect.app; moving that app into Applications is supported. To keep
the ZIP, uncheck Safari → Settings → General → Open “safe” files after
downloading. The reported ZIP-to-Trash behavior is consistent with archive
handling, not proof the app download failed. Do not infer a Gatekeeper diagnosis
from the Move to Trash / Done buttons without the exact warning text.

Sources: [GitHub direct latest-release links](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases),
[Safari downloads](https://support.apple.com/en-gb/guide/safari/sfri40598/mac),
[Safari General settings](https://support.apple.com/en-gb/guide/safari/ibrw1072/mac),
[Apple app security guidance](https://support.apple.com/en-gb/102445).
