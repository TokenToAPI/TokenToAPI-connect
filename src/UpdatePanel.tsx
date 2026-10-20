import { useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, CircleAlert, Download, LoaderCircle, RefreshCw } from "lucide-react";
import { nativeAvailable } from "./native";

export default function UpdatePanel({ busy, onInstalling }: {
  busy: boolean;
  onInstalling: (value: boolean) => void;
}) {
  const update = useRef<Update | null>(null);
  const checking = useRef(false);
  const installing = useRef(false);
  const panel = useRef<HTMLElement>(null);
  const checkAction = useRef<(automatic?: boolean) => Promise<void>>(async () => {});
  const didCheck = useRef(false);
  const mounted = useRef(true);
  const [phase, setPhase] = useState<"idle" | "checking" | "available" | "installing" | "installed">("idle");
  const [message, setMessage] = useState("");
  const [feedback, setFeedback] = useState<"success" | "error" | "">("");

  async function checkNow(automatic = false) {
    if (checking.current || installing.current || busy || phase === "installed" || !nativeAvailable || (automatic && didCheck.current)) return;
    if (!automatic) panel.current?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
    checking.current = true;
    didCheck.current = true;
    setPhase("checking");
    setMessage("");
    setFeedback("");
    try {
      const found = await check({ timeout: 15000 });
      if (!mounted.current) {
        await found?.close();
        return;
      }
      await update.current?.close();
      update.current = found;
      setPhase(found ? "available" : "idle");
      setFeedback(found || automatic ? "" : "success");
      setMessage(found ? `Connect ${found.version} is available.` : automatic ? "" : "You’re up to date.");
    } catch {
      if (mounted.current) {
        setPhase(update.current ? "available" : "idle");
        setFeedback(automatic ? "" : "error");
        setMessage(automatic ? "" : "Couldn’t check for updates. Check your connection and try again.");
      }
    } finally {
      checking.current = false;
    }
  }

  checkAction.current = checkNow;

  useEffect(() => {
    mounted.current = true;
    // Delay also avoids duplicate startup requests during development StrictMode.
    const timer = setTimeout(() => void checkAction.current(true), 1000);
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    if (nativeAvailable) {
      void listen("connect:check-for-updates", () => void checkAction.current()).then(stop => {
        if (cancelled) stop(); else unlisten = stop;
      }).catch(() => { /* The in-app button remains available if menu events fail. */ });
    }
    return () => {
      mounted.current = false;
      cancelled = true;
      unlisten?.();
      clearTimeout(timer);
      void update.current?.close();
      update.current = null;
    };
  }, []);

  async function install() {
    const selected = update.current;
    if (!selected || busy || checking.current || installing.current) return;
    installing.current = true;
    setFeedback("");
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
      installing.current = false;
      setFeedback("error");
      setPhase("available");
      setMessage("Update could not be installed. Try again, or download the latest release from tokentoapi.com.");
      onInstalling(false);
    }
  }

  if (!nativeAvailable) return null;
  const working = phase === "checking" || phase === "installing";
  const StatusIcon = feedback === "error" ? CircleAlert : feedback === "success" ? CheckCircle2 : Download;
  return <section ref={panel} className="update-panel" aria-label="App updates" data-feedback={feedback}>
    <span className={`update-status${message ? " is-visible" : ""}`} role="status" aria-live="polite">
      {message && <><StatusIcon size={15} aria-hidden="true" /><span>{message}</span></>}
    </span>
    {phase === "available" ? <button className="update-action is-primary" disabled={busy} onClick={() => void install()}><Download size={15} aria-hidden="true" />Update and restart</button>
      : phase === "installed" ? <button className="update-action is-primary" onClick={() => void relaunch().catch(() => setMessage("Please quit and reopen Connect to finish updating."))}><RefreshCw size={15} aria-hidden="true" />Restart Connect</button>
      : <button className="update-action" disabled={busy || working} onClick={() => void checkNow()}>
        {working ? <LoaderCircle className="update-spinner" size={15} aria-hidden="true" /> : <RefreshCw size={15} aria-hidden="true" />}
        {phase === "checking" ? "Checking…" : phase === "installing" ? "Updating…" : "Check for updates"}
      </button>}
  </section>;
}
