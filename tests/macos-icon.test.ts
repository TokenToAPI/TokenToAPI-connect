import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const icons = fileURLToPath(new URL("../src-tauri/icons/", import.meta.url));

function readPixels(png: string, bmp: string): Buffer {
  execFileSync("/usr/bin/sips", ["-s", "format", "bmp", png, "--out", bmp]);
  const bitmap = readFileSync(bmp);
  assert.equal(bitmap.readInt32LE(18), 32);
  assert.equal(bitmap.readInt32LE(22), -32);
  assert.equal(bitmap.readUInt16LE(28), 32);
  return bitmap.subarray(bitmap.readUInt32LE(10));
}

test("macOS extracts the Connect artwork at 32×32", { skip: process.platform !== "darwin" }, () => {
  const temp = mkdtempSync(join(tmpdir(), "connect-icon-test-"));
  try {
    const iconset = join(temp, "connect.iconset");
    execFileSync("/usr/bin/iconutil", [
      "-c", "iconset", "-o", iconset, process.env.CONNECT_MACOS_ICON ?? join(icons, "icon.icns"),
    ]);
    const actual = readPixels(join(iconset, "icon_32x32.png"), join(temp, "actual.bmp"));
    const expected = readPixels(join(icons, "32x32.png"), join(temp, "expected.bmp"));
    assert.equal(actual.length, expected.length);

    // Allow small differences in resizing and transparent edge pixels.
    const averageError = actual.reduce((sum, value, index) => sum + Math.abs(value - expected[index]), 0)
      / actual.length;
    assert.ok(averageError < 5, `32×32 icon differs from the artwork by ${averageError.toFixed(2)} levels per channel`);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
});
