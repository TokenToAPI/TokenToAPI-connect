import type { Client, ModelChoice } from "./native";
import { modelSelectable } from "./clients.ts";
import { EMPTY_SELECTION, selectModels } from "./selection.ts";

export function handoffSelection(client: Client, models: ModelChoice[]) {
  return selectModels(EMPTY_SELECTION, models.filter(model => modelSelectable(client, model.id)).map(model => model.id));
}
export function shouldAutoAccept(knownIssuer: boolean, busy: boolean, hasWork: boolean): boolean {
  return knownIssuer && !busy && !hasWork;
}
