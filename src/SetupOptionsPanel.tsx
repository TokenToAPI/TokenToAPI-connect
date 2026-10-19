import type { Client, SetupOptions } from "./native";

export default function SetupOptionsPanel({
  client,
  models,
  value,
  change,
  disabled,
}: {
  client: Client;
  models: string[];
  value: SetupOptions;
  change: (value: SetupOptions) => void;
  disabled: boolean;
}) {
  if (client.startsWith("codex")) {
    return (
      <div className="setup-options">
        <label className="capability-setting">
          <span>Web search</span>
          <select
            aria-label="Web search"
            value={value.webSearch}
            disabled={disabled}
            onChange={(event) =>
              change({
                ...value,
                webSearch: event.target.value as SetupOptions["webSearch"],
              })
            }
          >
            <option value="preserve">Preserve (off on first setup)</option>
            <option value="disabled">Disabled</option>
            <option value="cached">Cached</option>
            <option value="live">Live</option>
          </select>
        </label>
        <p>Search behavior depends on the gateway; live search may incur additional usage.</p>
      </div>
    );
  }
  if (client === "claude-desktop") return null;

  const roles = [
    ["haiku", "Background tasks / Haiku"],
    ["sonnet", "Sonnet alias"],
    ["opus", "Opus alias"],
    ["fable", "Fable alias"],
    ["subagent", "Subagent model"],
  ] as const;

  return (
    <details className="setup-options">
      <summary>Advanced Claude Code settings</summary>
      <p>Auto uses the unique selected model from that family; otherwise it uses the default model.</p>
      {roles.map(([role, label]) => (
        <label className="capability-setting" key={role}>
          <span>{label}</span>
          <select
            aria-label={label}
            value={value.claudeModels[role] ?? ""}
            disabled={disabled}
            onChange={(event) => {
              const claudeModels = { ...value.claudeModels };
              if (event.target.value) claudeModels[role] = event.target.value;
              else delete claudeModels[role];
              change({ ...value, claudeModels });
            }}
          >
            <option value="">{role === "subagent" ? "Follow session / subagent config" : "Auto-match selected models"}</option>
            {models.map((model) => (
              <option key={model} value={model}>{model}</option>
            ))}
          </select>
        </label>
      ))}
      <label className="capability-setting">
        <span>Artifact tool</span>
        <select
          aria-label="Artifact tool"
          value={value.artifactMode}
          disabled={disabled}
          onChange={(event) =>
            change({
              ...value,
              artifactMode: event.target.value as SetupOptions["artifactMode"],
            })
          }
        >
          <option value="preserve">Preserve existing setting</option>
          <option value="disabled">Disable</option>
          <option value="enabled">Allow</option>
        </select>
      </label>
    </details>
  );
}
