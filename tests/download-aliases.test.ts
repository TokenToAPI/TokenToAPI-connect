import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
// @ts-ignore Release helper is plain JavaScript.
import { prepareDownloadAliases } from '../scripts/prepare-download-aliases.mjs';

test('latest-download aliases preserve exact package bytes and update for successive versions', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'connect-alias-test-'));
  try {
    for (const version of ['0.2.1', '0.2.2']) {
      for (const suffix of ['macOS-arm64.zip', 'Windows-x64-setup.exe']) {
        const bytes = Buffer.from(`${version}:${suffix}`);
        fs.writeFileSync(path.join(dir, `TokenToAPI-Connect-${version}-${suffix}`), bytes);
      }
      const aliases = prepareDownloadAliases(dir, version);
      for (const alias of aliases) {
        const source = alias.replace('TokenToAPI-Connect-', `TokenToAPI-Connect-${version}-`);
        assert.deepEqual(fs.readFileSync(path.join(dir, alias)), fs.readFileSync(path.join(dir, source)));
        const digest = createHash('sha256').update(fs.readFileSync(path.join(dir, source))).digest('hex');
        assert.ok(fs.readFileSync(path.join(dir, 'DOWNLOADS-SHA256SUMS.txt'), 'utf8').includes(`${digest}  ${alias}`));
      }
    }
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});

test('missing or empty packages fail before any alias is changed', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'connect-alias-test-'));
  try {
    const alias = path.join(dir, 'TokenToAPI-Connect-macOS-arm64.zip');
    fs.writeFileSync(alias, 'previous');
    fs.writeFileSync(path.join(dir, 'TokenToAPI-Connect-0.2.2-macOS-arm64.zip'), 'new');
    assert.throws(() => prepareDownloadAliases(dir, '0.2.2'));
    assert.equal(fs.readFileSync(alias, 'utf8'), 'previous');
    fs.writeFileSync(path.join(dir, 'TokenToAPI-Connect-0.2.2-Windows-x64-setup.exe'), '');
    assert.throws(() => prepareDownloadAliases(dir, '0.2.2'));
    assert.equal(fs.readFileSync(alias, 'utf8'), 'previous');
    assert.throws(() => prepareDownloadAliases(dir, '../bad'));
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});
