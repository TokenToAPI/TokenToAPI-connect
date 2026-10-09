import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';

if (process.platform !== 'darwin') throw new Error('Build the macOS package on macOS.');
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const target = 'aarch64-apple-darwin';
function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${path.basename(command)} exited with ${result.status}`);
  return result.stdout;
}
run(process.execPath, [path.join(root, 'node_modules/@tauri-apps/cli/tauri.js'),
  'build', '--target', target, '--bundles', 'app', '--', '--locked']);

const source = path.join(root, 'src-tauri/target', target, 'release/bundle/macos/TokenToAPI Connect.app');
const actual = run('/usr/libexec/PlistBuddy', ['-c', 'Print :CFBundleShortVersionString', path.join(source, 'Contents/Info.plist')],
  { encoding: 'utf8', stdio: 'pipe' }).trim();
if (actual !== version) throw new Error('App version does not match package.json');
const binary = fs.readFileSync(path.join(source, 'Contents/MacOS/tokentoapi-connect'));
if (binary.readUInt32LE(0) !== 0xfeedfacf || binary.readUInt32LE(4) !== 0x0100000c) {
  throw new Error('Expected an Apple Silicon executable');
}
const release = path.join(root, 'release');
fs.mkdirSync(release, { recursive: true });
const stage = fs.mkdtempSync(path.join(os.tmpdir(), 'tokentoapi-macos-package-'));
try {
  const folder = path.join(stage, 'TokenToAPI Connect');
  fs.mkdirSync(folder);
  const app = path.join(folder, 'TokenToAPI Connect.app');
  run('/usr/bin/ditto', [source, app]);
  fs.copyFileSync(path.join(root, 'LICENSE'), path.join(app, 'Contents/Resources/LICENSE.txt'));
  fs.copyFileSync(path.join(root, 'LICENSE'), path.join(folder, 'LICENSE.txt'));
  fs.cpSync(path.join(root, 'licenses'), path.join(folder, 'licenses'), { recursive: true });
  run('/usr/bin/codesign', ['--force', '--deep', '--sign', '-', app]);
  run('/usr/bin/codesign', ['--verify', '--deep', '--strict', app]);
  fs.writeFileSync(path.join(folder, 'README.txt'), `TokenToAPI Connect ${version} · macOS Apple Silicon

After extracting, open TokenToAPI Connect.app; you can also drag it into the Applications folder.
Choose a gateway → choose a client → enter your API Key → fetch and select models → import.
After importing, fully quit the target client, then reopen it and start a new session.
To revert, click "Restore original configuration" in the top-right corner.

This package uses an ad-hoc signature and has not yet completed Apple Developer ID signing or notarization.
If the system blocks it, first confirm that you downloaded it from this project's GitHub Release, then follow the system prompt.

Source & docs: https://github.com/TokenToAPI/TokenToAPI-connect
Latest verification notes: https://github.com/TokenToAPI/TokenToAPI-connect/blob/main/VERIFICATION.md
`);
  const name = `TokenToAPI-Connect-${version}-macOS-arm64.zip`;
  const archive = path.join(stage, name);
  run('/usr/bin/ditto', ['-c', '-k', '--norsrc', '--keepParent', folder, archive]);
  fs.copyFileSync(archive, path.join(release, name));
  const hashes = fs.readdirSync(release).filter(name => name.endsWith('.zip')).sort()
    .map(name => `${createHash('sha256').update(fs.readFileSync(path.join(release, name))).digest('hex')}  ${name}`);
  fs.writeFileSync(path.join(release, 'SHA256SUMS.txt'), hashes.join('\n') + '\n');
  console.log(`Packaged: ${path.join(release, name)}`);
} finally {
  fs.rmSync(stage, { recursive: true, force: true });
}
