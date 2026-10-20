import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ check: vi.fn(), relaunch: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: mocks.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: mocks.relaunch }));
vi.mock("../../src/native", () => ({ nativeAvailable: true }));
import UpdatePanel from "../../src/UpdatePanel";
let root: Root, host: HTMLDivElement;
const installing = vi.fn();
const click = async (label: string) => {
  const button = [...host.querySelectorAll("button")].find(b => b.textContent === label)!;
  expect(button).toBeTruthy();
  await act(async () => button.click());
};
beforeEach(() => {
  (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
  vi.useFakeTimers();
  vi.clearAllMocks();
  mocks.relaunch.mockResolvedValue(undefined);
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.useRealTimers(); });
const render = async (busy = false) => { await act(async () => root.render(<UpdatePanel busy={busy} onInstalling={installing} />)); };
it("checks automatically, but never installs without a click", async () => {
  const update = { version: "0.1.9", downloadAndInstall: vi.fn(), close: vi.fn() };
  mocks.check.mockResolvedValue(update);
  await render();
  await act(async () => vi.advanceTimersByTimeAsync(1000));
  expect(host.textContent).toContain("0.1.9");
  expect(update.downloadAndInstall).not.toHaveBeenCalled();
  await render(true);
  expect(host.querySelector("button")?.disabled).toBe(true);
  await render(false);
  update.downloadAndInstall.mockImplementation(async callback => {
    callback({ event: "Started", data: { contentLength: 100 } });
    callback({ event: "Progress", data: { chunkLength: 50 } });
    callback({ event: "Finished" });
  });
  await click("Update and restart");
  expect(installing).toHaveBeenCalledWith(true);
  expect(mocks.relaunch).toHaveBeenCalledTimes(1);
});
it("keeps configuration usable after failed installation and allows retry", async () => {
  const update = { version: "0.1.9", downloadAndInstall: vi.fn().mockRejectedValue(new Error("signature rejected")), close: vi.fn() };
  mocks.check.mockResolvedValue(update);
  await render(); await click("Check for updates"); await click("Update and restart");
  expect(host.textContent).toContain("could not be installed");
  expect(mocks.relaunch).not.toHaveBeenCalled();
  expect(installing).toHaveBeenLastCalledWith(false);
  expect(host.querySelector("button")?.disabled).toBe(false);
});
it("handles no update and network failure without blocking setup", async () => {
  mocks.check.mockResolvedValue(null);
  await render(); await click("Check for updates");
  expect(host.textContent).toContain("up to date");
  mocks.check.mockRejectedValue(new Error("offline"));
  await click("Check for updates");
  expect(host.textContent).toContain("Couldn’t check");
  expect(installing).not.toHaveBeenCalled();
});
it("offers restart again when installation succeeds but relaunch fails", async () => {
  mocks.check.mockResolvedValue({ version: "0.1.9", downloadAndInstall: vi.fn().mockResolvedValue(undefined), close: vi.fn() });
  mocks.relaunch.mockRejectedValue(new Error("restart failed"));
  await render(); await click("Check for updates"); await click("Update and restart");
  expect(host.textContent).toContain("Update installed");
  await click("Restart Connect");
  expect(host.textContent).toContain("quit and reopen");
});
it("closes a late result after unmount", async () => {
  let finish!: (value: any) => void;
  mocks.check.mockImplementation(() => new Promise(resolve => { finish = resolve; }));
  await render(); await click("Check for updates");
  await act(async () => root.unmount());
  root = createRoot(host);
  const close = vi.fn();
  await act(async () => finish({ close }));
  expect(close).toHaveBeenCalledTimes(1);
});
