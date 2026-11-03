import type { Client, ModelChoice } from "./native";
import { modelSelectable } from "./clients.ts";
import { EMPTY_SELECTION, selectModels } from "./selection.ts";

export function handoffSelection(client: Client, models: ModelChoice[]) {
  const ids = models.filter(model => modelSelectable(client, model.id)).map(model => model.id);
  return selectModels(EMPTY_SELECTION, client === "gemini" ? ids.slice(0, 1) : ids);
}
export function shouldAutoAccept(knownIssuer: boolean, busy: boolean, hasWork: boolean): boolean {
  return knownIssuer && !busy && !hasWork;
}
