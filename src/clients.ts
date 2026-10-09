import type { Client } from "./native";

export const families = [
  { id: "codex", name: "Codex", maker: "OpenAI", hint: "Desktop app · Terminal" },
  { id: "claude", name: "Claude", maker: "Anthropic", hint: "Desktop app · Claude Code" },
] as const;

export const clients: { id: Client; name: string; label: string; hint: string }[] = [
  { id: "codex-desktop", name: "Codex Desktop", label: "Desktop", hint: "Local sessions, shares config with the terminal" },
  { id: "codex", name: "Codex Terminal", label: "Terminal", hint: "CLI, shares config with the desktop app" },
  { id: "claude-desktop", name: "Claude Desktop", label: "Desktop", hint: "Chat · Cowork · Local Code" },
  { id: "claude", name: "Claude Code Terminal", label: "Terminal", hint: "Claude Code CLI, also works in in-IDE terminals" },
  { id: "claude-vscode", name: "Claude Code · VS Code", label: "VS Code extension", hint: "VS Code default user config" },
];

export function clientFamily(client: Client): "codex" | "claude" {
  return client.startsWith("codex") ? "codex" : "claude";
}

export function modelSelectable(client: Client, id: string): boolean {
  return client !== "claude-desktop" || /^(anthropic\/)?claude-(sonnet|opus|haiku|fable)-[^\[]+$/i.test(id);
}
