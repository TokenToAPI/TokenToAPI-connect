# Client compatibility review

Reviewed on 2026-10-19 against the SuperGPT Connect v0.1.6 release notes and compatibility matrix. This review covers local client configuration behavior; it does not imply that every gateway implements every protocol feature.

| Upstream finding | Same risk in TokenToAPI Connect | v0.1.5 handling | Regression evidence |
| --- | --- | --- | --- |
| Codex credentials replacing official login state | Yes. The previous import wrote credentials to shared Codex auth state. | Bind the Bearer key to the TokenToAPI provider and leave `auth.json` and keyring preferences intact. | `config_roundtrip`, `codex_compatibility`, and the real Codex profile/runtime fixture. |
| Codex provider tables using reserved names | Yes. A pre-existing `openai`, `ollama`, or `lmstudio` table can shadow or conflict with generated providers. | Back up and remove conflicting reserved tables on import; restore the original bytes exactly. | `codex_reserved_provider_names_are_removed_and_restored_byte_for_byte`. |
| Web Search settings unexpectedly changing | Yes. Existing settings were not an explicit import choice. | Preserve a valid existing value or let the user choose disabled, cached, or live. New Codex setup defaults to disabled. | Rust setup-option/config tests and Codex runtime request checks. |
| Codex image metadata lost or media models offered as chat models | Yes. A plain model-name list cannot prove image support, and dedicated media models are not chat models. | Request the Codex model manifest with client version, fall back only when that optional request is rejected, preserve safe capability fields, and filter non-chat models. | `model_discovery` tests and the 0.162.0 URL/local-image runtime case. |
| Codex context and reasoning compatibility | Yes. Old generated catalogs could lose known capacity, numeric thinking levels, or Ultra mapping. | Retain valid local overrides, merge partial gateway limits, validate reasoning values, and repair the old 32K catalog. | `codex_compatibility` tests and the five real Codex runtime cases. |
| Claude Code inherited keys, OAuth, model locks, or cloud selectors | Yes. Clearing a settings file does not clear variables inherited by the client process. | Write explicit environment overrides for gateway authentication and neutralize conflicting inherited variables. | `claude_role_models_artifact_preference_and_inherited_environment_are_handled` and provider-field tests. |
| Claude Code purpose models and Artifact compatibility | Yes. Different roles may need distinct selected models and Artifact behavior must not be coupled to other options. | Support role-specific selected models, automatic family matching, subagent override, and preserve/disable/allow Artifact modes. | Setup-option and Claude config round-trip tests. |
| VS Code uses a separate Claude configuration directory or shows official login | Yes. A configured extension directory could differ from the CLI default. | Resolve only an absolute, unambiguous `CLAUDE_CONFIG_DIR` and use it consistently for preview, import, and restore; set the third-party login behavior. | `vscode_config_directory_controls_import_preview_and_restore_target` and VS Code round-trip tests. |
| Claude Desktop retains obsolete gateway fields or ambiguous profiles | Yes. Old local inference fields and duplicate profile IDs could make import ambiguous. | Remove only obsolete local gateway fields, preserve policy, and reject duplicate IDs before writing. | Claude Desktop round-trip and malformed-registry tests. |
| Windows Claude Desktop configuration directory has a suffix | Yes. Choosing among several candidates could overwrite the wrong directory. | Prefer the exact path, accept a single configured suffixed path, and refuse ambiguous candidates. | Windows-path selection unit tests; native Windows runtime remains unverified for this release. |
| Claude model picker minimum version | Yes. Older clients can accept configuration but cannot expose the same custom model menu. | Warn below Claude Code 2.1.242 using semantic version comparison. | Compatibility version unit tests. |
| MCP Tool Search auto-detection | Not established. SuperGPT reports a proprietary, per-route capability endpoint and service-side measurements. No equivalent TokenToAPI capability contract was found in this review. | Not copied or auto-enabled. This is a gateway capability question, not safe to infer from model discovery or successful authentication. | No service-level capability claim is made. |

The installed Claude Code on the development Mac is 2.1.88, below the custom-model-picker floor. Configuration generation is covered by tests, but no live Claude Code gateway conversation is claimed for v0.1.5. Windows configuration behavior is covered by platform-path tests and cross-compilation only unless a native Windows run is recorded in [VERIFICATION.md](VERIFICATION.md).

## v0.1.6 image-input follow-up

ID-only model discovery now inherits verified image support for the seven bundled GPT-6.x/GPT-5.6 native models. Explicit gateway modalities still take precedence; unsupported and duplicate modalities are filtered before writing Codex descriptors. Reimport with v0.1.6 repairs an older generated text-only catalog. Native Codex 0.162.1 tests now execute URL and local-file image turns for both explicit metadata and ID-only Astra fallback; the prior vision harness did not enter its runtime branch. See [verification](VERIFICATION.md) and the [recovery guide](docs/testing/codex-image-input.md).

## v0.1.7 complete-catalog follow-up

Capability handling is independent of a fixed list of model names: flat, nested provider, and native metadata work for aliases, verified bundled records cover 24 models, and the native cache fills gaps for newer names. Explicit gateway limits win. Missing or malformed capabilities produce a named warning; an unknown upstream is never assumed to support pictures. The live GPT catalog and the complete fallback matrix are covered in [VERIFICATION.md](VERIFICATION.md).

## Grok CLI and model routing (v0.1.9)

The official Grok Build CLI uses a separate TOML adapter, gateway-bound model
credentials, and selected-only model aliases. Context capacity is required from
the gateway; supported reasoning choices are translated into Grok's model menu.
Unrelated settings, permissions and MCP configuration are preserved. Generated
entries have an ownership marker, and collisions with unmanaged entries fail
without writes. Configuration and backups use private file permissions.

The adapter follows the official [custom models documentation](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/11-custom-models.md).
Native verification uses Grok 1.0.62 on macOS with a loopback gateway and fake
credentials. This validates client configuration and wire behavior, not paid
upstream inference. Native Grok on Windows is not independently exercised.
Command-line flags, environment overlays and managed organization policies may
still override local configuration.
