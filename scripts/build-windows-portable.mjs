// Build and package without an installer, development tools, or user configuration files.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const packageOnly = process.argv.includes('--package-only');
if (process.argv.slice(2).some(arg => arg !== '--package-only')) {
  throw new Error('Usage: node scripts/build-windows-portable.mjs [--package-only]');
}
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const target = 'x86_64-pc-windows-msvc';
function run(command, args, cwd = root) {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit', env: process.env });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${path.basename(command)} exited with ${result.status}`);
}
if (!packageOnly) {
  const args = [path.join(root, 'node_modules/@tauri-apps/cli/tauri.js'), 'build', '--target', target, '--no-bundle'];
  if (process.platform !== 'win32') args.push('--runner', 'cargo-xwin');
  args.push('--', '--locked');
  run(process.execPath, args);
}

const executable = path.join(root, 'src-tauri/target', target, 'release/tokentoapi-connect.exe');
const pe = fs.readFileSync(executable);
if (pe.length < 512 || pe.toString('ascii', 0, 2) !== 'MZ') throw new Error('Missing Windows PE executable');
if (!pe.includes(Buffer.from(version + '\0', 'utf16le'))) throw new Error('EXE version does not match package.json; rebuild first');
if (!pe.includes(Buffer.from('asInvoker'))) throw new Error('EXE must declare that administrator elevation is not required');
const header = pe.readUInt32LE(0x3c);
if (pe.readUInt32LE(header) !== 0x4550 || pe.readUInt16LE(header + 4) !== 0x8664) {
  throw new Error('The portable package requires a Windows x64 executable');
}
const optional = header + 24;
if (pe.readUInt16LE(optional) !== 0x20b || pe.readUInt16LE(optional + 68) !== 2) {
  throw new Error('Expected PE32+ with the Windows GUI subsystem');
}
const sections = header + 24 + pe.readUInt16LE(header + 20);
function rvaOffset(rva) {
  for (let i = 0; i < pe.readUInt16LE(header + 6); i++) {
    const section = sections + i * 40;
    const start = pe.readUInt32LE(section + 12);
    const size = Math.max(pe.readUInt32LE(section + 8), pe.readUInt32LE(section + 16));
    if (rva >= start && rva < start + size) return pe.readUInt32LE(section + 20) + rva - start;
  }
  throw new Error(`Invalid PE RVA ${rva}`);
}
const imports = [];
for (let offset = rvaOffset(pe.readUInt32LE(optional + 120)); pe.readUInt32LE(offset + 12); offset += 20) {
  const start = rvaOffset(pe.readUInt32LE(offset + 12));
  imports.push(pe.toString('ascii', start, pe.indexOf(0, start)));
}
const extraRuntime = imports.filter(name => /^(vcruntime|msvcp|concrt|ucrtbase|api-ms-win-crt|webview2loader)/i.test(name));
if (extraRuntime.length) throw new Error(`Portable EXE still requires unbundled runtime DLLs: ${extraRuntime.join(', ')}`);

const release = path.join(root, 'release');
fs.mkdirSync(release, { recursive: true });
const stage = fs.mkdtempSync(path.join(os.tmpdir(), 'tokentoapi-windows-package-'));
const folder = path.join(stage, 'TokenToAPI Connect');
const archiveName = `TokenToAPI-Connect-${version}-Windows-x64-portable.zip`;
try {
  fs.mkdirSync(folder);
  fs.copyFileSync(executable, path.join(folder, 'TokenToAPI Connect.exe'));
  fs.copyFileSync(path.join(root, 'LICENSE'), path.join(folder, 'LICENSE.txt'));
  fs.cpSync(path.join(root, 'licenses'), path.join(folder, 'licenses'), { recursive: true });
  const instructions = `TokenToAPI Connect ${version} · Windows portable edition

Works on: Windows 10 / 11, 64-bit Intel / AMD computers.

Getting started
1. First extract the entire archive into a folder.
2. Double-click "TokenToAPI Connect.exe". No administrator rights are needed, and no Node.js or Rust installation is required.
3. Choose a gateway → choose a client → enter your own API Key → fetch and select models → import.
4. Fully quit Claude / Codex, reopen it, and start a new session.
   If Claude stays in the system tray, quit it from the tray menu.
5. To revert, click "Restore original configuration" in the top-right corner, choose the client, then click "Restore".

If it won't open
This app uses the Microsoft Edge WebView2 Runtime. If it reports that it is missing, install the
x64 version of the Evergreen Standalone Installer from the Microsoft website, then reopen the app:
https://developer.microsoft.com/microsoft-edge/webview2/
WebView2 only needs to be installed when the system lacks this component; the connector itself requires no installation.
The current version has not undergone Windows publisher code signing, so the system may warn about an unknown publisher.

Confirm the configuration took effect
Check that the window, the model list, and the import behave correctly; after restarting the target client, send a test message,
then use "Restore original configuration" to check that you can return to the pre-import state. Seeing "Import successful" alone does not mean the client has switched.

Notes
The Claude / Codex clients must be installed separately. This tool only configures the local clients of the current Windows user;
WSL and remote development environments have their own configuration and must be set up in their own environments.
The app does not stay resident after closing. Configuration and restore records are stored in the current user directory; deleting the EXE does not restore the configuration automatically.
${process.platform === 'win32' ? 'This package was built on Windows.' : 'This package was cross-compiled on another system.'}Test scope:
https://github.com/TokenToAPI/TokenToAPI-connect/blob/main/VERIFICATION.md

Source & license: https://github.com/TokenToAPI/TokenToAPI-connect
`;
  // ASCII filenames also extract correctly in older Windows ZIP tools; write the file as UTF-8 with a BOM.
  fs.writeFileSync(path.join(folder, 'README.txt'), '\ufeff' + instructions.replace(/\n/g, '\r\n'));
  const stagedArchive = path.join(stage, archiveName);
  if (process.platform === 'darwin') {
    run('/usr/bin/ditto', ['-c', '-k', '--norsrc', '--keepParent', folder, stagedArchive]);
  } else if (process.platform === 'win32') {
    run('tar.exe', ['-a', '-cf', stagedArchive, 'TokenToAPI Connect'], stage);
  } else {
    run('zip', ['-q', '-r', stagedArchive, 'TokenToAPI Connect'], stage);
  }
  fs.copyFileSync(stagedArchive, path.join(release, archiveName));
  const hashes = fs.readdirSync(release).filter(name => name.endsWith('.zip')).sort()
    .map(name => `${createHash('sha256').update(fs.readFileSync(path.join(release, name))).digest('hex')}  ${name}`);
  fs.writeFileSync(path.join(release, 'SHA256SUMS.txt'), hashes.join('\n') + '\n');
  console.log(`Windows x64 GUI verified; no external VC++ or WebView2Loader DLL required.`);
  console.log(`Packaged: ${path.join(release, archiveName)}`);
  console.log(`Size: ${(fs.statSync(path.join(release, archiveName)).size / 1024 / 1024).toFixed(2)} MiB`);
} finally {
  fs.rmSync(stage, { recursive: true, force: true });
}
