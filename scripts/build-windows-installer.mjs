import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version;
const cli = path.join(root, 'node_modules/@tauri-apps/cli/tauri.js');
const args = [cli, 'build', '--target', 'x86_64-pc-windows-msvc', '--bundles', 'nsis'];
if (process.platform !== 'win32') args.push('--runner', 'cargo-xwin');
args.push('--', '--locked');
execFileSync(process.execPath, args, { cwd: root, stdio: 'inherit' });
const source = path.join(root, 'src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis');
const installers = fs.readdirSync(source).filter(name => name.includes(`_${version}_`) && name.endsWith('-setup.exe'));
if (installers.length !== 1) throw new Error('Expected exactly one Windows installer for this version');
fs.mkdirSync(path.join(root, 'release'), { recursive: true });
const dest = path.join(root, 'release', `TokenToAPI-Connect-${version}-Windows-x64-setup.exe`);
fs.copyFileSync(path.join(source, installers[0]), dest);
fs.copyFileSync(path.join(source, installers[0] + '.sig'), dest + '.sig');
