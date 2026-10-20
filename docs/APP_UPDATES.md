# App updates

Connect 0.1.8 and newer check the latest stable GitHub release shortly after startup.
Checks time out after 15 seconds and never send gateway credentials. Startup
network failures stay quiet; a manual check shows a retry message. Downloads
and installation start only when the user clicks **Update and restart**.
Import, restore, and other setup operations are disabled during installation.

The endpoint is
`https://github.com/TokenToAPI/TokenToAPI-connect/releases/latest/download/latest.json`.
Tauri verifies the embedded public key and the version in the signed package
before replacing the app. Older releases are not offered. Configuration and
backups live outside the application bundle and are preserved.

macOS uses the signed `.app.tar.gz` containing the same sealed app as the
manual ZIP. Install the app in a writable location, preferably Applications,
before updating. Windows uses a per-user NSIS installer; an update from the
portable edition installs a managed copy. Use its installed shortcut afterward
and remove the obsolete portable folder when no longer needed.

Manual checks are available in the bottom-right footer and in the native menu:
**TokenToAPI Connect → Check for updates…** on macOS and
**Help → Check for updates…** on Windows. Both use the same update flow and
show status in the footer. Checks are locked during imports and installation.

## Publishing

Keep the updater private key outside this repository in the team's credential
store. It must be retained for every future release. Do not regenerate it as
part of routine releases. Only its public counterpart belongs in app config.
Losing the private key prevents trusted updates to existing installations.

1. Bump package.json, Cargo.toml/Cargo.lock, and tauri.conf.json together. Add
   release notes in `docs/releases/vVERSION.md`.
2. Set `TAURI_SIGNING_PRIVATE_KEY` to the protected key file path and
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to its password (empty for an
   unencrypted key stored under filesystem access controls).
3. Run `pnpm build:macos`, `pnpm build:windows:installer`, then
   `pnpm package:windows`. Cross-building Windows requires cargo-xwin,
   LLVM/LLD, and NSIS. Native Windows builds use the same installer script.
4. Run `pnpm release:manifest`. It requires both signed updater artifacts.
   Upload the two manual ZIPs, Windows setup EXE, macOS updater archive,
   signatures, latest.json, and both checksum files to a draft GitHub release.
5. Run the regular tests and the macOS release-artifact test:
   `cargo test --manifest-path src-tauri/Cargo.toml --features release-tests --test updater_release`.
   Dispatch **Windows release smoke test** for the draft tag.
6. Verify uploaded bytes against checksums, then publish the release as latest.
   Confirm latest.json and both platform download URLs resolve publicly.
   Never publish a latest release without its manifest and both update packages.

The signing key does not replace Apple notarization or Windows publisher signing.

## Model refresh

**Refresh models** fetches the gateway again using the current setup session.
It resets the selection for review and does not write configuration until
**Import**. Gateway metadata and the native Codex cache allow newly advertised
capabilities without an app upgrade. Changes to Connect's behavior still need
an app update. Explicit text-only routes remain text-only.
