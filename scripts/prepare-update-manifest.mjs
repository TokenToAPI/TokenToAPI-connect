import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { prepareDownloadAliases } from './prepare-download-aliases.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version;
const release = path.join(root, 'release');
const prefix = `TokenToAPI-Connect-${version}-`;
const platforms = {};
for (const [target, suffix] of [['darwin-aarch64', 'macOS-arm64.app.tar.gz'], ['windows-x86_64', 'Windows-x64-setup.exe']]) {
  const filename = prefix + suffix;
  if (!fs.statSync(path.join(release, filename)).size) throw new Error(`Empty update: ${filename}`);
  const signature = fs.readFileSync(path.join(release, filename + '.sig'), 'utf8').trim();
  if (!signature || !Buffer.from(signature, 'base64').toString().startsWith('untrusted comment:')) {
    throw new Error(`Invalid signature file: ${filename}`);
  }
  platforms[target] = {
    signature,
    url: `https://github.com/TokenToAPI/TokenToAPI-connect/releases/download/v${version}/${filename}`,
  };
}
fs.writeFileSync(path.join(release, 'latest.json'), JSON.stringify({
  version,
  notes: fs.readFileSync(path.join(root, `docs/releases/v${version}.md`), 'utf8'),
  pub_date: new Date().toISOString(),
  platforms,
}, null, 2) + '\n');
const aliases = prepareDownloadAliases(release, version);
const files = fs.readdirSync(release).filter(name => name.startsWith(prefix) || aliases.includes(name) || name === 'latest.json' || name === 'DOWNLOADS-SHA256SUMS.txt').sort();
const checksums = files.map(name => `${createHash('sha256').update(fs.readFileSync(path.join(release, name))).digest('hex')}  ${name}`).join('\n') + '\n';
fs.writeFileSync(path.join(release, 'SHA256SUMS.txt'), checksums);
fs.writeFileSync(path.join(release, `SHA256SUMS-${version}.txt`), checksums);
console.log(`Prepared signed update manifest for ${version}: ${Object.keys(platforms).join(', ')}`);
