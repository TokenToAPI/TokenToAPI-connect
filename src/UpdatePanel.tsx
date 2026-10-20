import { useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { nativeAvailable } from "./native";

export default function UpdatePanel({ busy, onInstalling }: {
  busy: boolean;
  onInstalling: (value: boolean) => void;
}) {
  const update = useRef<Update | null>(null);
  const checking = useRef(false);
  const didCheck = useRef(false);
  const mounted = useRef(true);
  const [phase, setPhase] = useState<"idle" | "checking" | "available" | "installing" | "installed">("idle");
  const [message, setMessage] = useState("");

  async function checkNow(automatic = false) {
    if (checking.current || !nativeAvailable || (automatic && didCheck.current)) return;
    checking.current = true;
    didCheck.current = true;
    setPhase("checking");
    setMessage("");
    try {
      const found = await check({ timeout: 15000 });
      if (!mounted.current) {
        await found?.close();
        return;
      }
      await update.current?.close();
      update.current = found;
      setPhase(found ? "available" : "idle");
      setMessage(found ? `Connect ${found.version} is available.` : automatic ? "" : "You’re up to date.");
    } catch {
      if (mounted.current) {
        setPhase(update.current ? "available" : "idle");
        setMessage(automatic ? "" : "Couldn’t check for updates. Check your connection and try again.");
      }
    } finally {
      checking.current = false;
    }
  }

  useEffect(() => {
    mounted.current = true;
    // Delay also avoids duplicate startup requests during development StrictMode.
    const timer = setTimeout(() => void checkNow(true), 1000);
    return () => {
      mounted.current = false;
      clearTimeout(timer);
      void update.current?.close();
      update.current = null;
    };
  }, []);

  async function install() {
    const selected = update.current;
    if (!selected || busy || phase === "installing") return;
    onInstalling(true);
    setPhase("installing");
    setMessage("Downloading update…");
    let downloaded = 0;
    let total = 0;
    try {
      await selected.downloadAndInstall(event => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          setMessage(total
            ? `Downloading update… ${Math.min(100, Math.round(downloaded / total * 100))}%`
            : `Downloading update… ${(downloaded / 1048576).toFixed(1)} MB`);
        }
        if (event.event === "Finished") setMessage("Verifying and installing update…");
      });
      setPhase("installed");
      setMessage("Update installed. Restart Connect to finish.");
      try { await relaunch(); } catch { /* Keep the restart button available. */ }
    } catch {
      setPhase("available");
      setMessage("Update could not be installed. Try again, or download the latest release from tokentoapi.com.");
      onInstalling(false);
    }
  }

  if (!nativeAvailable) return null;
  return <section className="update-panel" aria-label="App updates">
    <span role="status" aria-live="polite">{message}</span>
    {phase === "available" ? <button disabled={busy} onClick={() => void install()}>Update and restart</button>
      : phase === "installed" ? <button onClick={() => void relaunch().catch(() => setMessage("Please quit and reopen Connect to finish updating."))}>Restart Connect</button>
      : <button disabled={busy || phase === "checking" || phase === "installing"} onClick={() => void checkNow()}>
        {phase === "checking" ? "Checking…" : phase === "installing" ? "Updating…" : "Check for updates"}
      </button>}
  </section>;
}
