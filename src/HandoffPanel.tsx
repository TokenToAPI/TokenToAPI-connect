import { useEffect, useRef } from "react";
import type { PendingHandoff, HandoffSetup } from "./native";

export default function HandoffPanel({ pending, setup, hasWork, busy, error, onAccept, onCancel }: {
  pending: PendingHandoff | null; setup: HandoffSetup | null; hasWork: boolean;
  busy: boolean; error: string; onAccept: () => void; onCancel: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { const node = dialog.current!; node.showModal(); return () => node.close(); }, []);
  return <dialog ref={dialog} className="station-dialog" aria-labelledby="handoff-title" onCancel={event => { event.preventDefault(); if (!busy) onCancel(); }}>
    <h2 id="handoff-title">{setup ? "Review the gateway" : "Import from the platform"}</h2>
    <p className="dialog-intro">{setup ? "This platform uses a different gateway. Confirm the destination before fetching models." : hasWork ? "You have setup in progress. Continuing will replace that setup with this platform import." : "Confirm the platform you want to receive this setup from."}</p>
    <p className="handoff-address">{setup?.baseUrl ?? pending?.issuer}</p>
    <p className="field-hint">Your client configuration will only change after you select models and click Import.</p>
    {error && <p role="alert" className="notice notice-error">{error}</p>}
    <div className="dialog-actions">
      <button className="secondary-button" onClick={onCancel} disabled={busy}>Cancel</button>
      <button className="primary-button" onClick={onAccept} disabled={busy}>{busy ? "Preparing…" : setup ? "Fetch models" : "Continue"}</button>
    </div>
  </dialog>;
}
