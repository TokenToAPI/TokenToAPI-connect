import type { Client } from "./native";

export const families = [
  { id: "codex", name: "Codex", maker: "OpenAI", hint: "Desktop app · Terminal" },
  { id: "claude", name: "Claude", maker: "Anthropic", hint: "Desktop app · Claude Code" },
  { id: "gemini", name: "Gemini", maker: "Google", hint: "Gemini CLI · Terminal" },
  { id: "grok", name: "Grok", maker: "xAI", hint: "Grok CLI · Terminal" },
] as const;

export const clients: { id: Client; name: string; label: string; hint: string }[] = [
  { id: "gemini", name: "Gemini CLI", label: "Terminal", hint: "Official Google Gemini CLI" },
  { id: "grok", name: "Grok CLI", label: "Terminal", hint: "Official Grok Build CLI" },
  { id: "codex-desktop", name: "Codex Desktop", label: "Desktop", hint: "Local sessions, shares config with the terminal" },
  { id: "codex", name: "Codex Terminal", label: "Terminal", hint: "CLI, shares config with the desktop app" },
  { id: "claude-desktop", name: "Claude Desktop", label: "Desktop", hint: "Chat · Cowork · Local Code" },
  { id: "claude", name: "Claude Code Terminal", label: "Terminal", hint: "Claude Code CLI, also works in in-IDE terminals" },
  { id: "claude-vscode", name: "Claude Code · VS Code", label: "VS Code extension", hint: "VS Code default user config" },
];

export function clientFamily(client: Client): "codex" | "claude" | "grok" | "gemini" {
  return client === "gemini" ? "gemini" : client === "grok" ? "grok" : client.startsWith("codex") ? "codex" : "claude";
}

export function modelSelectable(client: Client, id: string): boolean {
  if (/^(?:imagen-|veo-|lyria-|embedding-|gpt-image(?:-|$)|dall-e(?:-|$)|whisper-|tts-|text-embedding-|sora-|grok-imagine|grok-voice)/i.test(id.split("/").at(-1) ?? id)) return false;
  const name = id.split("/").at(-1) ?? id;
  if (/^gemini-/i.test(name) && /(?:image|tts|embedding|robotics|audio|live)/i.test(name)) return false;
  if (client === "gemini") return /^gemini-/i.test(id);
  return client !== "claude-desktop" || /^(anthropic\/)?claude-(sonnet|opus|haiku|fable)-[^\[]+$/i.test(id);
}
