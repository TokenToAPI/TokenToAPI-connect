import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

export const DOWNLOAD_SUFFIXES = ['macOS-arm64.zip', 'Windows-x64-setup.exe'];

export function prepareDownloadAliases(directory, version) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Invalid release version');
  const files = DOWNLOAD_SUFFIXES.map(suffix => ({
    source: `TokenToAPI-Connect-${version}-${suffix}`,
    alias: `TokenToAPI-Connect-${suffix}`,
  }));
  // Validate every input before changing aliases; never publish a partial pair.
  for (const { source } of files) {
    if (!fs.statSync(path.join(directory, source)).isFile() || !fs.statSync(path.join(directory, source)).size) {
      throw new Error(`Missing or empty release asset: ${source}`);
    }
  }
  const checksums = [];
  for (const { source, alias } of files) {
    fs.copyFileSync(path.join(directory, source), path.join(directory, alias));
    const digest = createHash('sha256').update(fs.readFileSync(path.join(directory, alias))).digest('hex');
    checksums.push(`${digest}  ${alias}`);
  }
  fs.writeFileSync(path.join(directory, 'DOWNLOADS-SHA256SUMS.txt'), checksums.join('\n') + '\n');
  return files.map(file => file.alias);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const version = process.argv[3] ?? JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version;
  console.log(prepareDownloadAliases(process.argv[2] ?? path.join(root, 'release'), version).join('\n'));
}
