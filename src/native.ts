import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export type Client =
  | "claude"
  | "codex"
  | "claude-desktop"
  | "claude-vscode"
  | "codex-desktop";
export interface ClientStatus {
  client: Client;
  hasBackup: boolean;
  pending: boolean;
  configExists: boolean;
  configPath: string;
  compatibility: {
    installed: boolean;
    version: string | null;
    warnings: string[];
    blocked: string | null;
  };
}
export interface Status {
  sandbox: boolean;
  clients: ClientStatus[];
}
export interface ImportResult {
  providerName?: string | null;
  client: Client;
  model: string;
  selectedModels: string[];
  baseUrl: string;
  changed: boolean;
  configPath: string;
  nextSteps: string[];
  warnings: string[];
}
export const nativeAvailable = isTauri();
export function readStatus(): Promise<Status> {
  return invoke("app_status");
}
export interface ModelChoice {
  id: string;
  name: string;
}
export interface DiscoveryResult {
  discoveryId: string;
  client: Client;
  baseUrl: string;
  models: ModelChoice[];
}
export function discoverModels(input: {
  client: Client;
  apiKey: string;
  baseUrl: string;
}): Promise<DiscoveryResult> {
  return invoke("discover_models", { input });
}
export function importConfig(input: {
  discoveryId: string;
  selectedModels: string[];
  defaultModel: string;
}): Promise<ImportResult> {
  return invoke("import_config", { input });
}
export interface RestoreFile {
  label: string;
  path: string;
  existedBefore: boolean;
  modifiedSinceImport: boolean;
}
export interface RestorePoint {
  client: Client;
  available: boolean;
  pending: boolean;
  createdAt: number | null;
  files: RestoreFile[];
  reviewId: string | null;
  error: string | null;
}
export interface RestoreResult {
  client: Client;
  restoredFiles: number;
  safetyBackupPath: string;
  wasPending: boolean;
}
export function readRestorePoints(): Promise<RestorePoint[]> {
  return invoke("restore_points");
}
export function restoreConfig(
  client: Client,
  reviewId: string,
  overwriteModified: boolean,
): Promise<RestoreResult> {
  return invoke("restore_config", { client, reviewId, overwriteModified });
}

export interface PendingHandoff {
  requestId: string;
  issuer: string;
  knownIssuer: boolean;
}
export interface HandoffSetup {
  requestId: string;
  providerName: string;
  baseUrl: string;
  suggestedClient: Client;
  allowedClients: Client[];
  gatewayNeedsReview: boolean;
}
export function pendingHandoff(): Promise<PendingHandoff | null> {
  return invoke("pending_handoff");
}
export function acceptHandoff(requestId: string): Promise<HandoffSetup> {
  return invoke("accept_handoff", { requestId });
}
export function discoverHandoffModels(requestId: string, client: Client): Promise<DiscoveryResult> {
  return invoke("discover_handoff_models", { requestId, client });
}
export function cancelHandoff(requestId: string): Promise<void> {
  return invoke("cancel_handoff", { requestId });
}
export async function watchHandoffs(callback: () => void): Promise<() => void> {
  return listen("connect-handoff", callback);
}
