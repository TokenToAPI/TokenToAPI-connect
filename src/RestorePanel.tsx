import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown, CircleAlert, LoaderCircle, X } from "lucide-react";
import {
  nativeAvailable,
  readRestorePoints,
  restoreConfig,
  type Client,
  type RestorePoint,
  type RestoreResult,
} from "./native";

const targets: { client: Client; name: string }[] = [
  { client: "claude-desktop", name: "Claude Desktop" },
  { client: "claude", name: "Claude Code · Terminal" },
  { client: "claude-vscode", name: "Claude Code · VS Code" },
  { client: "codex", name: "Codex · Desktop & terminal" },
];

export default function RestorePanel({
  initialClient,
  onClose,
  onRestored,
}: {
  initialClient: Client;
  onClose: () => void;
  onRestored: (client: Client) => Promise<void>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [points, setPoints] = useState<RestorePoint[]>([]);
  const [selected, setSelected] = useState<Client>(
    initialClient === "codex-desktop" ? "codex" : initialClient,
  );
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [overwrite, setOverwrite] = useState(false);
  const [result, setResult] = useState<Pick<
    RestoreResult,
    "client" | "wasPending"
  > | null>(null);
  const point = points.find((p) => p.client === selected);
  const choices = targets.filter((target) =>
    points.some((p) => p.client === target.client && (p.available || p.error)),
  );
  const target = targets.find((t) => t.client === selected)!;
  const changed =
    point?.files.some((file) => file.modifiedSinceImport) ?? false;

  async function refresh() {
    setLoading(true);
    setError("");
    setOverwrite(false);
    try {
      const found = nativeAvailable ? await readRestorePoints() : [];
      setPoints(found);
      setSelected((current) => {
        if (found.some((p) => p.client === current && (p.available || p.error)))
          return current;
        return (
          found.find((p) => p.available)?.client ??
          found.find((p) => p.error)?.client ??
          current
        );
      });
    } catch (error) {
      setError(
        typeof error === "string" ? error : "Config couldn't be checked right now. Please try again.",
      );
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    void refresh();
    return () => element?.close();
  }, []);

  async function restore() {
    if (
      busy ||
      loading ||
      !point?.available ||
      !point.reviewId ||
      (changed && !overwrite)
    )
      return;
    setBusy(true);
    setError("");
    try {
      const done = await restoreConfig(selected, point.reviewId, overwrite);
      setResult({ client: done.client, wasPending: done.wasPending });
      try {
        await onRestored(done.client);
      } catch {
        setError("Configuration restored. Reopen the app to refresh the status.");
      }
    } catch (error) {
      setError(
        typeof error === "string" ? error : "Restore didn't finish. Check again and retry.",
      );
    } finally {
      setBusy(false);
    }
  }

  const problem = error || point?.error;
  return (
    <dialog
      ref={dialog}
      className="station-dialog restore-dialog"
      aria-labelledby="restore-title"
      onCancel={(event) => {
        if (busy) event.preventDefault();
        else onClose();
      }}
    >
      <div className="restore-heading">
        <h2 id="restore-title">Restore original config</h2>
        <button
          className="icon-button"
          aria-label="Close restore window"
          onClick={onClose}
          disabled={busy}
        >
          <X size={18} />
        </button>
      </div>
      <div className="restore-body">
        {result ? (
          <div className="restore-complete" role="status">
            <span className="restore-done-icon">
              <Check size={22} />
            </span>
            <h3>{result.wasPending ? "Incomplete import reverted" : "Restored"}</h3>
            <p>
              {result.wasPending
                ? "You can keep restoring back to before the first import."
                : `Reopen ${target.name} to apply.`}
            </p>
          </div>
        ) : loading ? (
          <p className="restore-status" role="status">
            <LoaderCircle size={16} className="spinning" />
            Checking configuration…
          </p>
        ) : choices.length > 0 ? (
          <>
            <p className="restore-description">
              Undo the app's changes and return to before the import.
            </p>
            <label className="restore-client-label" htmlFor="restore-client">
              Choose a client
            </label>
            <div className="restore-client-control">
              <select
                id="restore-client"
                className="restore-client-select"
                value={selected}
                onChange={(event) => {
                  setSelected(event.target.value as Client);
                  setOverwrite(false);
                  setError("");
                }}
                disabled={busy}
              >
                {choices.map((item) => (
                  <option key={item.client} value={item.client}>
                    {item.name}
                  </option>
                ))}
              </select>
              <ChevronDown size={16} aria-hidden="true" />
            </div>
            {point?.available && (
              <>
                {point.pending && (
                  <p className="restore-hint">
                    The last import wasn't finished, so that change will be reverted first.
                  </p>
                )}
                {changed && (
                  <label className="restore-conflict">
                    <input
                      type="checkbox"
                      checked={overwrite}
                      onChange={(event) => setOverwrite(event.target.checked)}
                      disabled={busy}
                    />
                    <span>Also revert config changes made after the import</span>
                  </label>
                )}
                <p className="restore-hint">
                  Quit the client first, then reopen it after restoring.
                </p>
              </>
            )}
          </>
        ) : !problem ? (
          <p className="restore-status">
            {nativeAvailable
              ? "No config can be restored right now."
              : "Use the restore feature in the desktop app."}
          </p>
        ) : null}
        {problem && (
          <div className="notice notice-error" role="alert">
            <CircleAlert size={16} />
            <div>
              <p>{problem}</p>
              {!result && (
                <button
                  className="text-button restore-retry"
                  onClick={() => void refresh()}
                  disabled={busy || loading}
                >
                  Check again
                </button>
              )}
            </div>
          </div>
        )}
      </div>
      <div className="restore-actions">
        {result ? (
          <>
            {result.wasPending && (
              <button
                className="text-button"
                onClick={() => {
                  setResult(null);
                  void refresh();
                }}
                disabled={busy}
              >
                Keep restoring
              </button>
            )}
            <button
              className="primary-button"
              onClick={onClose}
              disabled={busy}
            >
              Done
            </button>
          </>
        ) : !loading && choices.length === 0 && !problem ? (
          <button className="primary-button" onClick={onClose}>
            Got it
          </button>
        ) : (
          <>
            <button
              className="secondary-button"
              onClick={onClose}
              disabled={busy}
            >
              Cancel
            </button>
            <button
              className="primary-button"
              onClick={restore}
              disabled={
                busy ||
                loading ||
                !point?.available ||
                !!problem ||
                (changed && !overwrite)
              }
            >
              {busy ? (
                <>
                  <LoaderCircle size={16} className="spinning" />
                  Restoring…
                </>
              ) : (
                "Restore"
              )}
            </button>
          </>
        )}
      </div>
    </dialog>
  );
}
