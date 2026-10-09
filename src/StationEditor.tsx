import { useEffect, useRef, useState, type FormEvent } from "react";
import { ArrowRight, CircleAlert, Globe2, Trash2, X } from "lucide-react";
import { normalizeStationUrl, type Station } from "./stations";

export default function StationEditor({
  station,
  stations,
  onSave,
  onDelete,
  onClose,
}: {
  station: Station | null;
  stations: Station[];
  onSave: (station: Station) => void;
  onDelete: (id: string) => void;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState(station?.name ?? "");
  const [baseUrl, setBaseUrl] = useState(station?.baseUrl ?? "");
  const [error, setError] = useState("");
  useEffect(() => {
    const element = dialog.current!;
    element.showModal();
    element.querySelector<HTMLInputElement>("#station-name")?.focus();
    return () => element.close();
  }, []);
  function submit(event: FormEvent) {
    event.preventDefault();
    try {
      const trimmed = name.trim();
      if (
        !trimmed ||
        trimmed.length > 32 ||
        /[\u0000-\u001f\u007f]/.test(trimmed)
      ) {
        throw new Error("Enter a gateway name of 1–32 characters.");
      }
      const normalized = normalizeStationUrl(baseUrl);
      if (
        stations.some(
          (item) => item.id !== station?.id && item.baseUrl === normalized,
        )
      ) {
        throw new Error("This API address is already in the list. Select the existing gateway instead.");
      }
      if (
        stations.some(
          (item) =>
            item.id !== station?.id &&
            item.name.toLowerCase() === trimmed.toLowerCase(),
        )
      ) {
        throw new Error("This name is already taken. Try a more distinct one.");
      }
      if (!station && stations.length >= 31)
        throw new Error("You can save up to 30 custom gateways. Delete one you no longer use first.");
      onSave({
        id: station?.id ?? `custom-${crypto.randomUUID()}`,
        name: trimmed,
        baseUrl: normalized,
      });
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Gateway details couldn't be saved. Check and try again.",
      );
    }
  }
  return (
    <dialog
      ref={dialog}
      className="station-dialog"
      aria-labelledby="station-dialog-title"
      onCancel={onClose}
    >
      <div className="dialog-heading">
        <span className="dialog-icon">
          <Globe2 size={22} />
        </span>
        <button
          className="icon-button"
          type="button"
          onClick={onClose}
          aria-label="Close gateway settings"
        >
          <X size={18} />
        </button>
      </div>
      <h2 id="station-dialog-title">{station ? "Edit gateway" : "Add gateway"}</h2>
      <p className="dialog-intro">Save the name and API address for easy selection next time.</p>
      <form onSubmit={submit} noValidate>
        <label htmlFor="station-name">Gateway name</label>
        <input
          id="station-name"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setError("");
          }}
          placeholder="e.g. My gateway"
          maxLength={32}
          autoComplete="off"
          autoFocus
        />
        <label htmlFor="station-url">API address</label>
        <input
          id="station-url"
          type="url"
          value={baseUrl}
          onChange={(e) => {
            setBaseUrl(e.target.value);
            setError("");
          }}
          placeholder="https://api.example.com"
          maxLength={2048}
          spellCheck={false}
          autoCapitalize="none"
          autoComplete="off"
        />
        <p className="field-hint">
          Enter the API base address from your provider; /v1 is allowed.
        </p>
        {error && (
          <div className="notice notice-error dialog-error" role="alert">
            <CircleAlert size={16} />
            <p>{error}</p>
          </div>
        )}
        {station && (
          <p className="delete-hint">
            Deleting only removes the list entry; imported client config isn't affected.
          </p>
        )}
        <div className="dialog-actions">
          {station && (
            <button
              type="button"
              className="delete-button"
              onClick={() => onDelete(station.id)}
            >
              <Trash2 size={15} />
              Delete
            </button>
          )}
          <button type="button" className="secondary-button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="primary-button">
            {station ? "Save changes" : "Save and select"}
            <ArrowRight size={16} />
          </button>
        </div>
      </form>
    </dialog>
  );
}
