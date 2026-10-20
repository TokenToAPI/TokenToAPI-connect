import type { Client, SetupOptions } from "./native";

export const DEFAULT_OPTIONS: SetupOptions = {
  claudeModels: {},
  artifactMode: "preserve",
  webSearch: "preserve",
};

export function selectedOptions(
  client: Client,
  models: string[],
  options: SetupOptions,
): SetupOptions {
  if (client.startsWith("codex")) {
    return { ...DEFAULT_OPTIONS, webSearch: options.webSearch };
  }
  if ((client === "claude-desktop" || client === "grok")) return DEFAULT_OPTIONS;
  return {
    ...options,
    webSearch: "preserve",
    claudeModels: Object.fromEntries(
      Object.entries(options.claudeModels).filter(
        ([role, model]) =>
          ["haiku", "sonnet", "opus", "fable", "subagent"].includes(role) &&
          models.includes(model),
      ),
    ),
  };
}
