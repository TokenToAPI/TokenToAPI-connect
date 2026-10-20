# Codex image attachment rejection

Reported symptom: Codex displays “This model does not support image inputs. Try a different model” with GPT-6 Astra selected after importing through Connect.

## Cause and fix

Connect v0.1.5 requests `/v1/models?client_version=0.162.0` first and falls back to plain `/v1/models` for gateways rejecting the query. Either endpoint can return ID-only entries. A standard response can contain only model IDs, while the richer Codex manifest supplies input modalities. Connect v0.1.4/v0.1.5 initializes each imported descriptor with `input_modalities: ["text"]` and its bundled profiles do not restore image support. An ID-only Astra entry therefore produces a local catalog that blocks image attachments.

The source fix adds verified input modalities to seven bundled GPT-6.x/GPT-5.6 profiles and copies those capabilities before applying gateway metadata. Explicit gateway modalities still override the fallback. Unknown models remain text-only. The modality values were checked against the installed native Codex model cache on 2026-10-20; no provider inference was performed.

This reproduces the reported behavior, but the affected user's catalog and API response have not yet been inspected. An explicitly text-only gateway response could cause the same symptom and should be corrected at the gateway rather than overridden globally.

## Existing-user workaround

1. Fully quit Codex.
2. Open the catalog referenced by `model_catalog_json` in the user's Codex `config.toml`. Connect normally writes `tokentoapi-models.json` in the same Codex directory (`~/.codex` on macOS/Linux or `%USERPROFILE%\.codex` on Windows; custom Codex homes may differ).
3. Back up the catalog. Find the model entry with `slug: "gpt-6-astra"` and set its `input_modalities` to `["text", "image"]`. Keep the rest of the JSON intact. Only do this for a route known to support image input.
4. Reopen Codex, start a new chat, select Astra, and attach a small PNG. If attachment works but sending fails, investigate the gateway's image request handling separately.

Reimporting with v0.1.4/v0.1.5 can restore the faulty text-only entry. Install Connect v0.1.6, import again and fully restart Codex.

Do not request or share `auth.json`, API keys, or a complete configuration containing credentials. For diagnosis, the affected model's catalog row and the Connect/Codex versions are sufficient.

## Validation

Regression tests cover ID-only discovery for the seven verified native vision models, case-insensitive profile matching, and explicit gateway text-only precedence. Existing tests cover unknown-model fallback and malformed modality sanitization.

Release validation passed 90 Rust tests and six native Codex 0.162.1 scenarios, including URL/local PNG sends for Astra with ID-only discovery, stale text-only catalog repair, repeated import stability, and exact restore. Mac and Windows packages were built; Windows was cross-compiled without a native Windows launch. See [VERIFICATION.md](../../VERIFICATION.md). The affected user's installation has not been tested directly.
