import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type ReactNode,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ChevronRight,
  CircleAlert,
  Eye,
  EyeOff,
  Globe2,
  KeyRound,
  LoaderCircle,
  LockKeyhole,
  Moon,
  Pencil,
  Plus,
  RotateCcw,
  Search,
  ShieldCheck,
  Sun,
  Terminal,
  X,
} from "lucide-react";
import {
  discoverModels,
  importConfig,
  nativeAvailable,
  readStatus,
  type Client,
  type DiscoveryResult,
  type ImportResult,
  type Status,
} from "./native";
import {
  DEFAULT_STATION,
  loadStationPreferences,
  saveStationPreferences,
  stationAddress,
  type Station,
  type StationPreferences,
} from "./stations";
import {
  EMPTY_SELECTION,
  MAX_MODELS,
  selectModels,
  type ModelSelection,
} from "./selection";
import StationEditor from "./StationEditor";
import RestorePanel from "./RestorePanel";
import { version } from "../package.json";

const steps = ["Gateway", "Client", "API Key", "Select models"];
import { clients, families, clientFamily, modelSelectable } from "./clients";
function BrandIcon({ className = "" }: { className?: string }) {
  return (
    <img
      className={`brand-icon ${className}`}
      src="/tokentoapi.png"
      alt=""
      draggable={false}
    />
  );
}
function ClientIcon({ client, size = 24 }: { client: Client; size?: number }) {
  return clientFamily(client) === "codex" ? (
    <Terminal size={size} strokeWidth={1.6} />
  ) : (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
    >
      {[0, 30, 60, 90, 120, 150].map((angle) => (
        <path
          key={angle}
          d="M12 2.5V21.5"
          stroke="currentColor"
          strokeWidth="1.8"
          transform={`rotate(${angle} 12 12)`}
        />
      ))}
    </svg>
  );
}
function Modal({
  title,
  children,
  close,
}: {
  title: string;
  children: ReactNode;
  close: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const node = ref.current!;
    node.showModal();
    return () => node.close();
  }, []);
  return (
    <dialog
      ref={ref}
      className="station-dialog"
      aria-label={title}
      onCancel={close}
    >
      <div className="modal-title">
        <h2>{title}</h2>
        <button className="icon-button" onClick={close} aria-label="Close dialog">
          <X size={18} />
        </button>
      </div>
      {children}
    </dialog>
  );
}
function initialTheme(): "light" | "dark" {
  try {
    return localStorage.getItem("tokentoapi-connect.theme") === "dark"
      ? "dark"
      : "light";
  } catch {
    return "light";
  }
}
export default function App() {
  const [theme, setTheme] = useState(initialTheme);
  const [step, setStep] = useState(0);
  const [furthest, setFurthest] = useState(0);
  const [preferences, setPreferences] = useState<StationPreferences>(() => ({
    ...loadStationPreferences(),
    selected: DEFAULT_STATION.id,
  }));
  const [editing, setEditing] = useState<Station | null | undefined>();
  const [client, setClient] = useState<Client>("codex-desktop");
  const [apiKey, setApiKey] = useState("");
  const [visible, setVisible] = useState(false);
  const [discovery, setDiscovery] = useState<DiscoveryResult | null>(null);
  const [selection, setSelection] = useState<ModelSelection>(EMPTY_SELECTION);
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState<Status | null>(null);
  const [busy, setBusy] = useState<"discover" | "import" | "restore" | null>(
    null,
  );
  const [error, setError] = useState("");
  const [result, setResult] = useState<ImportResult | null>(null);
  const [restored, setRestored] = useState(false);
  const [showRestore, setShowRestore] = useState(false);
  const [about, setAbout] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const stations = [DEFAULT_STATION, ...preferences.custom];
  const station =
    stations.find((item) => item.id === preferences.selected) ??
    DEFAULT_STATION;
  const selectedClient = clients.find((item) => item.id === client)!;
  const clientStatus = status?.clients.find((item) => item.client === client);
  const disabled = busy !== null;
  const family = clientFamily(client);
  const matching =
    discovery?.models.filter((model) =>
      `${model.id} ${model.name}`
        .toLowerCase()
        .includes(query.trim().toLowerCase()),
    ) ?? [];
  const selectableMatching = matching.filter((model) =>
    modelSelectable(client, model.id),
  );
  const allMatchingSelected =
    selectableMatching.length > 0 &&
    selectableMatching.every((model) => selection.ids.includes(model.id));

  async function refresh() {
    if (nativeAvailable) setStatus(await readStatus());
  }
  useEffect(() => {
    refresh().catch(() => setError("Unable to read client status. Please reopen the app."));
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem("tokentoapi-connect.theme", theme);
    } catch {
      /* Session theme still works. */
    }
  }, [theme]);
  useEffect(() => {
    window.scrollTo({ top: 0 });
    heading.current?.focus({ preventScroll: true });
  }, [step, result]);
  function clearDiscovery() {
    setDiscovery(null);
    setSelection(EMPTY_SELECTION);
    setQuery("");
    setError("");
    setResult(null);
    setRestored(false);
  }
  function changePreferences(next: StationPreferences) {
    clearDiscovery();
    setApiKey("");
    setVisible(false);
    setFurthest(0);
    setPreferences(next);
    if (!saveStationPreferences(next))
      setError("The gateway was selected for this session, but couldn't be saved on this device.");
  }
  function saveStation(next: Station) {
    changePreferences({
      version: 1,
      selected: next.id,
      custom: preferences.custom.some((item) => item.id === next.id)
        ? preferences.custom.map((item) => (item.id === next.id ? next : item))
        : [...preferences.custom, next],
    });
    setEditing(undefined);
  }
  function changeClient(next: Client) {
    if (next === client) return;
    setClient(next);
    clearDiscovery();
    setApiKey("");
    setVisible(false);
    setFurthest(1);
  }
  function go(next: number) {
    setStep(next);
    setFurthest((current) => Math.max(current, next));
    setError("");
  }
  async function fetchModels(event?: FormEvent) {
    event?.preventDefault();
    if (disabled || !apiKey.trim()) return;
    clearDiscovery();
    if (!nativeAvailable) {
      setError("This is a UI preview. Fetch models and import from the desktop app.");
      return;
    }
    setBusy("discover");
    try {
      const fetched = await discoverModels({
        client,
        baseUrl: station.baseUrl,
        apiKey: apiKey.trim(),
      });
      setDiscovery(fetched);
      go(3);
    } catch (error) {
      setError(typeof error === "string" ? error : "Fetch failed. Please try again later.");
    } finally {
      setBusy(null);
    }
  }
  async function importSelected() {
    if (disabled || !discovery || !selection.ids.length) return;
    setBusy("import");
    setError("");
    try {
      const imported = await importConfig({
        discoveryId: discovery.discoveryId,
        selectedModels: selection.ids,
        defaultModel: selection.defaultId,
      });
      setResult(imported);
      setApiKey("");
      setVisible(false);
      setDiscovery(null);
      try {
        await refresh();
      } catch {
        setError("Config imported, but status refresh failed. Reopen the app to view the backup.");
      }
    } catch (error) {
      setError(typeof error === "string" ? error : "Import failed. Please try again later.");
    } finally {
      setBusy(null);
    }
  }
  async function restoredClient(next: Client) {
    clearDiscovery();
    setClient(next === "codex" ? "codex-desktop" : next);
    setApiKey("");
    setVisible(false);
    setRestored(true);
    setStep(0);
    setFurthest(0);
    await refresh();
  }
  function startAgain() {
    clearDiscovery();
    setApiKey("");
    setVisible(false);
    setStep(0);
    setFurthest(0);
    setPreferences((current) => ({ ...current, selected: DEFAULT_STATION.id }));
  }
  function toggleModel(id: string, checked: boolean) {
    setSelection((current) =>
      selectModels(
        current,
        checked
          ? [...current.ids, id]
          : current.ids.filter((value) => value !== id),
      ),
    );
  }
  return (
    <main className={`app-shell ${step === 1 ? "client-step" : ""}`}>
      <header className="app-header">
        <div className="brand">
          <BrandIcon />
          <span className="brand-wordmark">tokentoapi</span>
          <span className="brand-divider" />
          <span className="brand-product">Connect</span>
        </div>
        <div className="header-actions">
          <button
            className="restore-header-button"
            onClick={() => setShowRestore(true)}
            disabled={disabled}
          >
            <RotateCcw size={15} />
            <span>Restore original config</span>
          </button>
          <button
            className="icon-button theme-button"
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
            aria-label={theme === "light" ? "Switch to dark appearance" : "Switch to light appearance"}
            title={theme === "light" ? "Dark appearance" : "Light appearance"}
          >
            {theme === "light" ? <Moon size={17} /> : <Sun size={17} />}
          </button>
        </div>
      </header>
      <div className="workspace">
        <div className="page-intro">
          <div>
            <h1>Connect your AI tools</h1>
            <p>From key to models, configured your way.</p>
          </div>
          <span className="local-label">
            <span />
            Local config
          </span>
        </div>
        {(!nativeAvailable || status?.sandbox) && (
          <div className="preview-label">
            {!nativeAvailable
              ? "UI preview · use the desktop app to fetch models and import"
              : "Test mode · using an isolated config directory"}
          </div>
        )}
        <nav className="stepper" aria-label="Setup steps">
          <ol>
            {steps.map((title, index) => (
              <li
                key={title}
                className={`${step === index && !result ? "current" : ""} ${index < step || result ? "complete" : ""}`}
              >
                <button
                  disabled={
                    disabled ||
                    !!result ||
                    index > furthest ||
                    (index === 3 && !discovery)
                  }
                  onClick={() => go(index)}
                  aria-current={step === index && !result ? "step" : undefined}
                >
                  <span className="step-circle">
                    {index < step || result ? (
                      <Check size={13} strokeWidth={2.5} />
                    ) : (
                      index + 1
                    )}
                  </span>
                  <span>{title}</span>
                </button>
              </li>
            ))}
          </ol>
        </nav>
        <section
          className={`wizard-card ${step === 3 ? "models-card" : ""}`}
          aria-labelledby="step-title"
          aria-busy={disabled}
        >
          {result ? (
            <div className="success-content" role="status">
              <span className="success-mark">
                <Check size={28} strokeWidth={1.7} />
              </span>
              <h2 id="step-title" ref={heading} tabIndex={-1}>
                Configuration written. Restart the client to apply.
              </h2>
              <p>{selectedClient.name} · gateway and selected models saved</p>
              <div className="result-summary">
                <div>
                  <span>Gateway</span>
                  <strong>{station.name}</strong>
                </div>
                <div>
                  <span>Default model</span>
                  <strong className="mono">{result.model}</strong>
                </div>
                <div>
                  <span>Imported models</span>
                  <strong>{result.selectedModels.length}</strong>
                </div>
              </div>
              <div className="result-models">
                {result.selectedModels.map((id) => (
                  <span key={id}>{id}</span>
                ))}
              </div>
              <div className="activation-guide">
                <strong>Next steps</strong>
                <ol>
                  {result.nextSteps.map((text) => (
                    <li key={text}>{text}</li>
                  ))}
                </ol>
                <p>Config write verified; the client actually making requests hasn't been verified yet.</p>
              </div>
              {result.warnings.length > 0 && (
                <details className="compatibility-details">
                  <summary>Compatibility notes</summary>
                  {result.warnings.map((text) => (
                    <p key={text}>{text}</p>
                  ))}
                </details>
              )}
            </div>
          ) : (
            <>
              <div className="card-heading">
                <span className="eyebrow">
                  STEP 0{step + 1} <span>/ 04</span>
                </span>
                <h2 id="step-title" ref={heading} tabIndex={-1}>
                  {
                    [
                      "Choose a gateway",
                      "Choose the client to import to",
                      "Enter your API key",
                      "Choose the models to import",
                    ][step]
                  }
                </h2>
                <p>
                  {
                    [
                      "TokenToAPI is the default, but you can choose another gateway.",
                      "Pick the product first, then the entry point you actually use.",
                      `Use a key from ${station.name} to fetch the models you can choose from.`,
                      "Check the models you need and set a default.",
                    ][step]
                  }
                </p>
              </div>
              {step === 0 && (
                <div className="step-content station-content">
                  <fieldset className="station-list" disabled={disabled}>
                    <legend className="sr-only">Choose a gateway (single select)</legend>
                    {stations.map((item) => (
                      <div
                        key={item.id}
                        className={`station-row ${station.id === item.id ? "is-selected" : ""}`}
                      >
                        <label className="station-option">
                          <input
                            type="radio"
                            name="station"
                            value={item.id}
                            checked={station.id === item.id}
                            onChange={() => {
                              if (item.id !== station.id)
                                changePreferences({
                                  ...preferences,
                                  selected: item.id,
                                });
                            }}
                          />
                          <span className="station-emblem">
                            {item.id === DEFAULT_STATION.id ? (
                              <BrandIcon />
                            ) : (
                              <Globe2 size={24} strokeWidth={1.5} />
                            )}
                          </span>
                          <span className="station-copy">
                            <span className="station-name">
                              {item.name}
                              {item.id === DEFAULT_STATION.id && (
                                <span className="badge">Default</span>
                              )}
                            </span>
                            <span className="station-address">
                              {stationAddress(item)}
                            </span>
                          </span>
                          <span className="radio-indicator" aria-hidden="true">
                            <span />
                          </span>
                        </label>
                        {item.id !== DEFAULT_STATION.id && (
                          <button
                            className="icon-button edit-station"
                            aria-label={`Edit ${item.name}`}
                            onClick={() => setEditing(item)}
                            disabled={disabled}
                          >
                            <Pencil size={15} />
                          </button>
                        )}
                      </div>
                    ))}
                  </fieldset>
                  <button
                    className="add-station"
                    onClick={() => setEditing(null)}
                    disabled={disabled}
                  >
                    <span className="add-icon">
                      <Plus size={20} strokeWidth={1.5} />
                    </span>
                    <span>
                      <strong>Add another gateway</strong>
                      <small>Enter a name and API address</small>
                    </span>
                    <ChevronRight size={17} />
                  </button>
                </div>
              )}
              {step === 1 && (
                <div className="step-content">
                  <fieldset className="client-grid" disabled={disabled}>
                    <legend className="sr-only">Choose a client</legend>
                    {families.map((item) => (
                      <label
                        key={item.id}
                        className={`client-option ${family === item.id ? "is-selected" : ""}`}
                      >
                        <input
                          type="radio"
                          name="client"
                          checked={family === item.id}
                          onChange={() =>
                            changeClient(
                              item.id === "codex"
                                ? "codex-desktop"
                                : "claude-desktop",
                            )
                          }
                          value={item.id}
                        />
                        <span className="client-top">
                          <span className="client-icon">
                            <ClientIcon client={item.id} size={27} />
                          </span>
                          <span className="radio-indicator" aria-hidden="true">
                            <span />
                          </span>
                        </span>
                        <span className="client-name">{item.name}</span>
                        <span className="client-maker">{item.maker}</span>
                        <span className="client-hint">{item.hint}</span>
                      </label>
                    ))}
                  </fieldset>
                  <fieldset className="edition-list" disabled={disabled}>
                    <legend>Which entry point do you use?</legend>
                    {clients
                      .filter((item) => clientFamily(item.id) === family)
                      .map((item) => (
                        <label
                          key={item.id}
                          className={`edition-option ${client === item.id ? "is-selected" : ""}`}
                        >
                          <input
                            type="radio"
                            name="edition"
                            checked={client === item.id}
                            onChange={() => changeClient(item.id)}
                          />
                          <span>
                            <strong>{item.label}</strong>
                            <small>{item.hint}</small>
                          </span>
                          <span className="edition-check">
                            {client === item.id && <Check size={15} />}
                          </span>
                        </label>
                      ))}
                  </fieldset>
                  <div className="client-detection">
                    <span
                      className={`detection-dot ${clientStatus?.compatibility.installed ? "found" : ""}`}
                    />
                    <span>
                      {!nativeAvailable
                        ? "The desktop app will detect installations automatically"
                        : !clientStatus
                          ? "Checking for the client…"
                          : clientStatus.compatibility.installed
                            ? `Detected ${selectedClient.name}${clientStatus.compatibility.version ? ` ${clientStatus.compatibility.version}` : ""}`
                            : "Not found in the usual locations; you can still save the config"}
                    </span>
                  </div>
                  {clientStatus?.compatibility.blocked && (
                    <p className="inline-note blocking-note">
                      {clientStatus.compatibility.blocked}
                    </p>
                  )}
                  {!!clientStatus?.compatibility.warnings.length && (
                    <details className="compatibility-details">
                      <summary>View compatibility notes</summary>
                      {clientStatus.compatibility.warnings.map((text) => (
                        <p key={text}>{text}</p>
                      ))}
                    </details>
                  )}
                  <p className="inline-note">
                    Only local entry points are configured; Claude
                    web, mobile, and cloud tasks don't read local config.
                  </p>
                </div>
              )}
              {step === 2 && (
                <form
                  id="key-form"
                  className="step-content key-content"
                  onSubmit={fetchModels}
                >
                  <div className="connection-route">
                    <span className="route-station">
                      {station.id === DEFAULT_STATION.id ? (
                        <BrandIcon />
                      ) : (
                        <Globe2 size={22} />
                      )}
                      <strong>{station.name}</strong>
                    </span>
                    <ArrowRight size={17} />
                    <span>
                      <ClientIcon client={client} size={20} />
                      <strong>{selectedClient.name}</strong>
                    </span>
                  </div>
                  <label className="field-label" htmlFor="api-key">
                    API Key
                  </label>
                  <div className="secret-field">
                    <KeyRound size={18} />
                    <input
                      id="api-key"
                      name="api-key"
                      type={visible ? "text" : "password"}
                      value={apiKey}
                      onChange={(event) => {
                        setApiKey(event.target.value);
                        clearDiscovery();
                        setFurthest(2);
                      }}
                      placeholder="Paste your API key"
                      autoComplete="off"
                      autoCapitalize="none"
                      spellCheck={false}
                      maxLength={4096}
                      disabled={disabled}
                      aria-describedby="key-destination"
                    />
                    <button
                      type="button"
                      className="icon-button"
                      aria-label={visible ? "Hide key" : "Show key"}
                      aria-pressed={visible}
                      onClick={() => setVisible(!visible)}
                      disabled={disabled}
                    >
                      {visible ? <EyeOff size={17} /> : <Eye size={17} />}
                    </button>
                  </div>
                  <p id="key-destination" className="key-destination">
                    <LockKeyhole size={13} />
                    <span>
                      The key is only sent to <b>{stationAddress(station)}</b>
                    </span>
                  </p>
                  <div className="key-note">
                    <span className="note-number">Next</span>
                    <p>
                      Fetches the model list for this key,
                      <br />
                      and lets you decide which models to import.
                    </p>
                  </div>
                </form>
              )}
              {step === 3 && discovery && (
                <div className="step-content model-content">
                  <div className="model-toolbar">
                    <div className="search-field">
                      <Search size={16} />
                      <input
                        aria-label="Search models"
                        placeholder="Search model names…"
                        value={query}
                        onChange={(event) => setQuery(event.target.value)}
                        disabled={disabled}
                        autoComplete="off"
                      />
                    </div>
                    <span className="model-count">
                      {discovery.models.length} models
                    </span>
                  </div>
                  <div className="list-heading">
                    <span>
                      {query ? `${matching.length} found` : "Available models"}
                    </span>
                    <button
                      className="text-button"
                      disabled={disabled || !selectableMatching.length}
                      onClick={() =>
                        setSelection((current) =>
                          selectModels(
                            current,
                            allMatchingSelected
                              ? current.ids.filter(
                                  (id) =>
                                    !selectableMatching.some(
                                      (model) => model.id === id,
                                    ),
                                )
                              : [
                                  ...current.ids,
                                  ...selectableMatching.map(
                                    (model) => model.id,
                                  ),
                                ],
                          ),
                        )
                      }
                    >
                      {allMatchingSelected
                        ? "Clear selection"
                        : query
                          ? "Select search results"
                          : "Select all"}
                    </button>
                  </div>
                  <div
                    className="model-list"
                    role="group"
                    aria-label="Check the models to import"
                  >
                    {matching.map((model) => {
                      const checked = selection.ids.includes(model.id);
                      const isDefault = model.id === selection.defaultId;
                      return (
                        <div
                          key={model.id}
                          className={`model-row ${checked ? "is-checked" : ""}`}
                        >
                          <label>
                            <input
                              type="checkbox"
                              checked={checked}
                              disabled={
                                !modelSelectable(client, model.id) ||
                                disabled ||
                                (!checked && selection.ids.length >= MAX_MODELS)
                              }
                              onChange={(event) =>
                                toggleModel(model.id, event.target.checked)
                              }
                              aria-label={`Import ${model.id}`}
                            />
                            <span className="checkbox-mark" aria-hidden="true">
                              {checked && <Check size={12} strokeWidth={2.7} />}
                            </span>
                            <span className="model-name">
                              <strong>{model.name}</strong>
                              {model.name !== model.id && (
                                <small>{model.id}</small>
                              )}
                            </span>
                          </label>
                          {!modelSelectable(client, model.id) && (
                            <span className="model-unavailable">
                              Not compatible with the desktop app
                            </span>
                          )}
                          {checked && (
                            <button
                              className={`default-model ${isDefault ? "active" : ""}`}
                              aria-label={`Set ${model.id} as the default model`}
                              aria-pressed={isDefault}
                              onClick={() =>
                                setSelection((current) => ({
                                  ...current,
                                  defaultId: model.id,
                                }))
                              }
                              disabled={disabled}
                            >
                              {isDefault ? (
                                <>
                                  <span />
                                  Default model
                                </>
                              ) : (
                                "Set as default"
                              )}
                            </button>
                          )}
                        </div>
                      );
                    })}
                    {!matching.length && (
                      <div className="empty-models">
                        <Search size={22} />
                        <p>No matching models found</p>
                        <button
                          className="text-button"
                          onClick={() => setQuery("")}
                        >
                          Clear search
                        </button>
                      </div>
                    )}
                  </div>
                  <div className="selection-summary" aria-live="polite">
                    <span>
                      Selected <strong>{selection.ids.length}</strong> models
                      {selection.ids.length >= MAX_MODELS && " (limit reached)"}
                    </span>
                    {selection.ids.length > 0 ? (
                      <span className="default-summary">
                        Default:
                        <b title={selection.defaultId}>{selection.defaultId}</b>
                      </span>
                    ) : (
                      <span>Select at least one</span>
                    )}
                  </div>
                  <p className="fine-print">
                    The model list comes from this key; requests require the gateway to support the{" "}
                    {family === "codex" ? "Responses" : "Messages"} API.
                  </p>
                </div>
              )}
            </>
          )}
          {(error || clientStatus?.pending || restored) && (
            <div className="feedback-area">
              {clientStatus?.pending && (
                <div className="notice notice-error" role="alert">
                  <CircleAlert size={16} />
                  <p>The last import was interrupted. Restore the config from the entry below first.</p>
                </div>
              )}
              {error && (
                <div className="notice notice-error" role="alert">
                  <CircleAlert size={16} />
                  <p>{error}</p>
                  <button
                    className="icon-button"
                    aria-label="Dismiss notice"
                    onClick={() => setError("")}
                  >
                    <X size={15} />
                  </button>
                </div>
              )}
              {restored && (
                <div className="notice notice-success" role="status">
                  <Check size={17} />
                  <p>Restored {selectedClient.name} to its pre-import configuration.</p>
                </div>
              )}
            </div>
          )}
          <div className="card-actions">
            {result ? (
              <>
                <span className="action-hint">
                  <ShieldCheck size={14} />
                  Original config backed up
                </span>
                <button className="primary-button" onClick={startAgain}>
                  Configure another client
                  <ArrowRight size={16} />
                </button>
              </>
            ) : (
              <>
                {step > 0 ? (
                  <button
                    className="back-button"
                    onClick={() => go(step - 1)}
                    disabled={disabled}
                  >
                    <ArrowLeft size={15} />
                    Back
                  </button>
                ) : (
                  <span className="action-hint">Each connection uses one gateway</span>
                )}
                {step < 2 && (
                  <button
                    className="primary-button"
                    disabled={
                      disabled ||
                      (step === 1 && !!clientStatus?.compatibility.blocked)
                    }
                    onClick={() => go(step + 1)}
                  >
                    Next
                    <ArrowRight size={16} />
                  </button>
                )}
                {step === 2 && (
                  <button
                    type="submit"
                    form="key-form"
                    className="primary-button"
                    disabled={disabled || !apiKey.trim()}
                  >
                    {busy === "discover" ? (
                      <>
                        <LoaderCircle className="spinning" size={16} />
                        Fetching models…
                      </>
                    ) : (
                      <>
                        Fetch available models
                        <ArrowRight size={16} />
                      </>
                    )}
                  </button>
                )}
                {step === 3 && (
                  <button
                    className="primary-button"
                    disabled={
                      disabled ||
                      !selection.ids.length ||
                      !discovery ||
                      !!clientStatus?.pending ||
                      !!clientStatus?.compatibility.blocked
                    }
                    onClick={importSelected}
                  >
                    {busy === "import" ? (
                      <>
                        <LoaderCircle className="spinning" size={16} />
                        Importing…
                      </>
                    ) : (
                      <>
                        Import to {selectedClient.name}
                        <ArrowRight size={16} />
                      </>
                    )}
                  </button>
                )}
              </>
            )}
          </div>
        </section>
        <div className="below-card">
          <span>
            <ShieldCheck size={14} />
            Original config backed up automatically before import
          </span>
          {clientStatus?.hasBackup && (
            <button
              className="text-button"
              onClick={() => setShowRestore(true)}
              disabled={disabled}
            >
              <RotateCcw size={13} />
              Restore {selectedClient.name} config
            </button>
          )}
        </div>
      </div>
      <footer className="app-footer">
        <span>TokenToAPI Connect</span>
        <button onClick={() => setAbout(true)} className="version-button">
          v{version} <span>·</span> About
        </button>
      </footer>
      {editing !== undefined && (
        <StationEditor
          station={editing}
          stations={stations}
          onSave={saveStation}
          onDelete={(id) => {
            changePreferences({
              version: 1,
              selected: DEFAULT_STATION.id,
              custom: preferences.custom.filter((item) => item.id !== id),
            });
            setEditing(undefined);
          }}
          onClose={() => setEditing(undefined)}
        />
      )}
      {showRestore && (
        <RestorePanel
          initialClient={client}
          onClose={() => setShowRestore(false)}
          onRestored={restoredClient}
        />
      )}
      {about && (
        <Modal title="TokenToAPI Connect" close={() => setAbout(false)}>
          <p className="dialog-intro">v{version} · local API configurator</p>
          <p className="about-copy">
            Built on CC Switch's config engine and model adaptation logic, MIT licensed; the full license ships with the app.
          </p>
          <p className="about-copy">
            Only gateway names and addresses are stored. Keys are used only for requests to the selected gateway and client config; they're never written to browser storage.
          </p>
          <div className="dialog-actions">
            <button className="primary-button" onClick={() => setAbout(false)}>
              Got it
            </button>
          </div>
        </Modal>
      )}
    </main>
  );
}
