# Mac acceptance: Kendrick and YC

Use the **same** v0.1.3 macOS ZIP and matching deployed platform revision on
both Macs. Each person completes their own record. This is the acceptance gate
before Windows work, separate from automated local fixture checks.

## Before testing

- Confirm Apple Silicon and macOS 12 or later; record the actual macOS version.
- Record Connect's source commit, v0.1.3 package SHA-256, and deployed platform
  source commit. Compare `shasum -a 256 TokenToAPI-Connect-0.1.3-macOS-arm64.zip`
  with the shipped `SHA256SUMS-0.1.3.txt`.
- Fully quit older Connect copies, extract the candidate, move it to
  Applications if desired, and open it once. Check About shows v0.1.3.
- Keep a copy of any client configuration you cannot afford to lose. Quit the
  target client before import. Use a test API key with a suitable active group.
- Use Microsoft Edge for the shared baseline; record other browsers separately.

## Import and request checks

1. Quit Connect. Open platform **API Keys**, choose **Import to Connect**, then
   **Open TokenToAPI Connect**. Accept the browser's app-opening prompt.
   Expected: Connect starts and fills the gateway/client without asking you
   to copy a key or address. A recognized production issuer can fetch directly;
   unfamiliar issuers and differing gateways require review.
2. Check the gateway, available clients and model list match the selected key.
   Pick models and a default, then click **Import**. Expected: a success screen
   names the correct gateway, client and selected models.
3. Fully quit/reopen the target client. Start a new **local** session and send
   one small test request. Expected: the selected model works through your
   TokenToAPI gateway and the platform records usage for that key. This may
   incur normal usage charges; run it manually with your own test key.
4. Leave Connect running and prepare a fresh platform import. Expected: the
   same app receives it; no second instance or manual key entry is needed.
   Existing setup must be reviewed before replacement.
5. Try a mixed-group key, if available, and switch between permitted Claude
   and Codex entry points. A Claude Code-only key must not offer Claude Desktop
   or Codex. Fetch again after switching; never reuse another client's catalog.

## Recovery and layout checks

- Keep an app-opening link unused for over two minutes. Opening it must give
  a retry instruction. Prepare a new link on the platform and confirm it works.
- Reopen a consumed link. It must not import the key a second time.
- Cancel a pending import. Existing setup/configuration must remain intact.
- Choose **Use original config** and confirm restoration. Later local edits
  require explicit review instead of silent overwrite. Reopen the client to
  apply the restored configuration.
- While the restore chooser is open, send another platform handoff. It must
  remain queued until restore review is closed, then continue safely.
- Check equal spacing between the default gateway, **Use original config**,
  **Add another gateway**, and any saved gateway cards; check light and dark
  appearance at the app's normal window size.
- Missing app: use the platform's Mac download, open it once, return to the
  platform, prepare a fresh link, and retry. v0.1.2 is not a handoff-compatible
  substitute. Manual setup remains available.

## Acceptance confirmation

On 2026-10-12, Kendrick confirmed that both he and YC tested the deployed update and that everything works. Both overall Mac acceptance results are accepted based on that report; detailed environment versions and individual checklist results were not supplied.

## Result record

| Field | Kendrick | YC |
| --- | --- | --- |
| Date | Pending | Pending |
| Mac model / macOS | Pending | Pending |
| Browser version | Pending | Pending |
| Connect source commit / ZIP SHA-256 | Pending | Pending |
| Deployed platform source commit | Pending | Pending |
| Client / version / group type | Pending | Pending |
| Cold and running-app imports | Pending | Pending |
| Real local request / platform usage | Pending | Pending |
| Expiry / retry / cancellation | Pending | Pending |
| Restore / queued handoff | Pending | Pending |
| Home-page spacing | Pending | Pending |
| Overall result | Passed — user reported | Passed — reported by Kendrick |

Report exact steps, versions, and a redacted screenshot/error when a check
fails. Never put API keys, handoff links/codes, authentication files, or restore
records into GitHub issues. Keep results Pending until that person tests them.

## Reminder after both Macs are finished

Before starting Windows work, remind Kendrick to request:
**“Clean up the Mac testing artifacts and Docker test resources now.”**
Use [the recorded cleanup inventory](post-macos-cleanup.md), confirm both test
results are saved, and preserve the accepted release and real configurations.
