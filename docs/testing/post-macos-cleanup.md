# Cleanup reminder after Kendrick and YC finish testing

Status: **cleanup completed on 2026-10-12**. Kendrick confirmed both Mac acceptance results and explicitly requested cleanup after Windows publication and platform deployment. Preserve the accepted release and real configurations. Removed task files totaling about 15.9 GiB (toolchains, both merged test worktrees and temporary fixtures/downloads/logs/binaries), plus the three Docker test images totaling about 542.6 MB. Docker now reports zero images, containers, volumes and build cache. Filesystem/Docker VM reclamation may differ from these logical sizes. The primary checkouts, published Mac/Windows ZIPs, installed applications, credentials and real client configurations were preserved.

## Follow-up audit — 2026-10-13

The first pass left generated files in the primary platform checkout and a task-created shared package cache. The follow-up removes approximately another 1.4 GiB: frontend `node_modules` (725 MiB), embedded frontend build output (8.3 MiB), pnpm v3 store created during this task (662 MiB), pnpm metadata created during this task (21 MiB), and the isolated `com.tokentoapi.connect.test` WebKit/cache data. Source remains clean.

Docker was started for verification and again reported zero images, containers, volumes and build cache; it was then stopped. Its small VM disk is Docker application infrastructure, not a retained test image. Pre-existing pnpm v10/v11 caches, Codex/Hugging Face caches, normal Connect app data and release downloads are preserved because they are shared or unrelated to this task.

## Inventory recorded before publication (2026-10-12)

Paths below are relative to Kendrick's `Startup-Bois` workspace unless stated
otherwise. Sizes are a snapshot, not a guaranteed amount recoverable later.

| Resource | Observed size / purpose | Cleanup scope |
| --- | --- | --- |
| `.toolchains/` | 9.1 GB; temporary Go/Rust tools and caches | Remove task-created tools/caches after confirming they are no longer needed for Windows or other work |
| `.worktrees/t2a-connect/src-tauri/target/` | 5.4 GB; Rust debug/release builds, test apps and DMGs | Remove generated outputs after retaining the accepted release asset |
| `.worktrees/t2a-platform/sub2api/frontend/node_modules/` | 712 MB | Remove with the accepted feature worktree |
| `.worktrees/t2a-connect/node_modules/` | 163 MB | Remove with the accepted feature worktree |
| Feature worktrees under `.worktrees/` | Temporary source checkouts | Confirm merged, pushed and free of user edits; remove using `git worktree remove` |
| Temporary `t2a-*` files/directories in macOS temp locations | Logs, fake-key homes, unpacked candidates, scripts and verification binaries | Inspect names and remove only this session's artifacts |
| Duplicate candidate ZIPs and extracted test app copies | Candidate packaging / smoke checks | Retain the accepted published ZIP/checksum and the user's installed app; remove redundant test copies |

The temporary mock gateway and Vite servers were stopped after verification.
The separately named `TokenToAPI Connect Test.app` was closed and removed.
The user's normal Connect app and real client configurations were preserved.

## Docker inventory

At this snapshot: **0 containers, 0 volumes, 0 build-cache bytes**, and about
**542.6 MB** of test images:

- `postgres:18.1-alpine3.23` — image `aa6eb304ddb6` (396 MB)
- `redis:8.4-alpine` — image `0514fa59e3d8` (132 MB)
- `testcontainers/ryuk:0.13.0` — image `31b31269d066` (13.8 MB)

Check current references before removing these specific images. Remove any
remaining containers/volumes only if their labels establish that they belong
to these integration-test runs. Avoid broad `docker system prune -a --volumes`:
it can delete unrelated project data. Docker's VM disk may reclaim host space
later than the logical image deletion; report both Docker and host disk use.

Credentials, SSH keys, production data, real Claude/Codex configuration, final
release assets and unrelated Docker resources are outside this cleanup list.
