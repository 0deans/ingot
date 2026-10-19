**Rust refactor plan for `src-tauri`**

Prepared on 2026-10-04 from the current working tree. This document is a plan; no application code was changed. Existing frontend edits were left intact. File sizes include comments, blank lines, and inline tests.

The recommended approach is an incremental refactor within the existing crate: organize code by feature, make application services independent of Tauri, introduce typed errors and validated inputs, and add deterministic tests before moving complex behavior. Separate mechanical moves from changes to failure behavior. A workspace split or a general dependency-injection framework is unnecessary at this stage.

Agents performing the refactor must first read [the Rust coding rules](C:/Users/denus/Desktop/ingot/src-tauri/AGENTS.md). The [root instruction file](C:/Users/denus/Desktop/ingot/AGENTS.md) directs repository-root tasks to those rules, including related frontend and CI edits. This plan defines what to change and in what order; the instruction files define how to implement and verify each change. Instruction placement follows the official [AGENTS.md guidance](https://learn.chatgpt.com/docs/agent-configuration/agents-md).

**Observed baseline**

- 52 Rust source files under `src-tauri/src`, totaling 25,168 lines.
- Six files exceed 1,000 lines. The largest is `ipc.rs`, at 2,826 lines.
- The single `AppApi` trait contains 140 commands and 8 events.
- 63 test declarations exist across 23 source files. Six tests are explicitly ignored. Five other tests call live external APIs and run by default.
- A text search finds 526 lines containing single-line `Result<..., String>` signatures. This is an indicator of pervasive string errors, not a count of distinct functions or defects.
- `thiserror` and `anyhow` appear transitively in Cargo.lock, but neither is a direct dependency or the application's error-handling convention.
- Rust TauRPC is 0.8.2; the installed frontend package is 2.0.0. Its generated proxy currently uses `errorHandling: "throw"`.

Verification performed from `src-tauri`:

```text
cargo test --lib --locked --offline -- --skip minecraft::content::tests --skip auth::ely::tests::test_fetch_catalog_with_uploader

52 passed; 0 failed; 6 ignored; 5 filtered out.
```

This verifies the selected existing tests on Windows. It does not measure coverage, exercise the five live API tests, or verify Linux/macOS/Android behavior. Some selected tests inspect the local OS or temporary files; they are not all pure unit tests.

The current [CI workflow](C:/Users/denus/Desktop/ingot/.github/workflows/ci.yml:84) runs the full library suite only on Linux. Windows and macOS run the keyring subset plus an explicit native-vault test. There is no coverage gate or Rust formatting/Clippy job in that workflow. Android builds are a separate manually triggered workflow.

**What should change, and why**

| Current file | Lines | Responsibilities to separate | Proposed files within its feature |
| --- | ---: | --- | --- |
| `ipc.rs` | 2,826 | Procedure declarations, resolvers, process singletons, HTTP client, events, lifecycle, version-change orchestration, transfer handling | `ipc/` feature adapters; `app/state.rs`, `app/lifecycle.rs`, `app/events.rs`; move workflows into their owning services |
| `minecraft/content.rs` | 1,505 | Two provider APIs, query translation, response conversion, search merging, downloads, two modpack formats | `content/{models,search,install}.rs`, `content/providers/{modrinth,curseforge}.rs`, `content/modpacks/{modrinth,curseforge}.rs` |
| `minecraft/importer.rs` | 1,153 | Platform discovery, five launcher formats, version parsing, copy policy, execution | `importer/{models,discovery,version_id,plan,execute}.rs`, `importer/formats/{curseforge,modrinth,atlauncher,vanilla,legacy}.rs` |
| `server/map.rs` | 1,133 | World discovery, Java regions, Bedrock reads, live chunks, block colors, rasterization, caching | `map/{models,discovery,java,bedrock,live_chunks,palette,raster,cache}.rs` |
| `version_change/mod.rs` | 1,027 | Models, scanning, compatibility decisions, dependency resolution, downloads, filesystem transaction, backups | `version_change/{models,scan,planner,dependencies,staging,transaction,journal,backup,error}.rs`; retain `targets`, `sources`, and `crash` |
| `minecraft/launcher.rs` | 1,024 | Preparation, authentication, argument construction, native extraction, progress, process ownership, exit actions | `launcher/{models,prepare,arguments,quick_play,progress,process,exit}.rs` |
| `server/process.rs` | 965 | Start reservation, installation, platform command construction, process registry, console queries, log parsing, stop/kill | `process/{manager,lifecycle,launch,command,console,log_parser,shutdown}.rs` |
| `server/live.rs` | 964 | SNBT parser, NBT representation, item/player conversion, console queries, offline data | `live/{nbt,snbt,models,items,players,query,storage}.rs` |
| `server/plugins.rs` | 924 | Provider APIs, compatibility, dependency installation, JAR metadata, tracking, update checks | `plugins/{models,platform,metadata,tracking,install,updates}.rs` plus provider adapters |
| `account.rs` | 803 | Profile persistence, login orchestration, refresh, account selection, skins, downloads | `account/{models,repository,service,session,skins,error}.rs` |
| `server/config.rs` | 803 | Domain models, JSON repository, properties, ports, whitelist/UUID repair, icons | `config/{models,repository,service,ports,properties,access,icons}.rs` |
| `auth/ely.rs` | 726 | Auth API, web login/OAuth flow, skin catalog and upload, HTML/URL helpers | `auth/ely/{models,auth,web_session,catalog,skins}.rs` |
| `minecraft/screenshots.rs` | 608 | Discovery, platform locations, thumbnail caching, deletion, reveal/open | `screenshots/{models,discovery,thumbnails,service}.rs` with platform opening at the application boundary |
| `minecraft/sync.rs` | 503 | Effective settings, synchronization decisions, file operations, status/conflicts | `sync/{models,policy,execute,service}.rs` |

These are destination responsibilities, not a requirement to create every suggested file immediately. Keep cohesive small modules together. Aim for most implementation files to be roughly 150–350 lines; review a file when it grows beyond about 500. Keep `mod.rs` files mostly declarations and a small public surface. Do not split stable modules merely to meet a number, or move every struct into one global `types.rs`.

`launch_minecraft` currently spans roughly 740 lines by itself, and `launch_server_inner` roughly 470. Splitting those files without extracting preparation, decisions, and lifecycle ownership would leave the hardest code unchanged.

**Concrete failure behavior to address with regression tests**

These findings come from source inspection. They are specific risks to reproduce and fix; successful baseline tests do not exercise these failure paths.

| Evidence | Consequence | Planned behavior and regression |
| --- | --- | --- |
| [Account loading](C:/Users/denus/Desktop/ingot/src-tauri/src/account.rs:65) defaults malformed JSON to an empty vector | Corrupt storage can appear to mean no accounts; a later save can replace it | Distinguish missing, malformed, and unreadable storage. Preserve corrupt input and return a typed error. Test all three cases. |
| [Importer](C:/Users/denus/Desktop/ingot/src-tauri/src/minecraft/importer.rs:951) ignores many copy errors, then returns `success: true` | A partially copied instance can be reported as fully imported | Produce an import plan, stage required files, publish only after success. Explicitly report optional failures and copied counts. |
| [Version-change rollback](C:/Users/denus/Desktop/ingot/src-tauri/src/version_change/mod.rs:855) discards restore errors and removes the backup | The message claims nothing changed even when recovery failed | Preserve recovery material, return both primary and recovery failure, and test failures at each mutation step. |
| [Journal persistence](C:/Users/denus/Desktop/ingot/src-tauri/src/version_change/mod.rs:859) happens after the swap; [instance metadata](C:/Users/denus/Desktop/ingot/src-tauri/src/ipc.rs:1198) and [server metadata](C:/Users/denus/Desktop/ingot/src-tauri/src/ipc.rs:2539) are saved separately | Files, tracking data, and configured versions can disagree after a write failure or interruption | Give one service ownership of apply/undo and metadata commit, with a durable, recoverable transaction protocol. Test metadata failure and restart recovery. |
| [Modpack extraction](C:/Users/denus/Desktop/ingot/src-tauri/src/minecraft/content.rs:1275) and [Bedrock pack extraction](C:/Users/denus/Desktop/ingot/src-tauri/src/server/bedrock_packs.rs:421) join archive-derived paths directly | Containment rules differ between importers | Share validated archive-relative paths and a contained extractor. Test traversal, Windows path forms, links, and malformed archives. Server transfer already uses `enclosed_name`; reuse the approach and extend its checks. |
| [Download cache validation](C:/Users/denus/Desktop/ingot/src-tauri/src/minecraft/downloader.rs:18) returns on matching length before checking an available checksum | A same-size corrupt file can count as cached | Verify supplied checksums, stage downloads, and publish verified artifacts. Test same-size corruption and interrupted downloads. |
| [Plugin installation](C:/Users/denus/Desktop/ingot/src-tauri/src/server/plugins.rs:713) removes old files before the new download succeeds; tracking writes can be ignored | Failed updates can remove a working installation or lose its tracking | Download and validate first; replace files and tracking with recoverable failure handling. Test failed download and failed tracking commit. |
| [Combined search](C:/Users/denus/Desktop/ingot/src-tauri/src/minecraft/content.rs:555) ignores provider failures; source adapters also use `Option` for failed requests | An outage can look like no results or no compatible release | Use `Result<Option<T>, ProviderError>` where absence is valid. Return explicit partial-result warnings or a typed failure when all providers fail. |
| [Launcher](C:/Users/denus/Desktop/ingot/src-tauri/src/minecraft/launcher.rs:833) and [server process](C:/Users/denus/Desktop/ingot/src-tauri/src/server/process.rs:947) call back into `ipc` | Business logic depends on its transport layer; tests need application scaffolding | Inject event callbacks and application actions; let the app layer coordinate window/quit behavior. |

Keep these fixes in focused changes after characterization tests. Do not accidentally preserve an unsafe behavior as the desired specification. Also do not convert every ignored error into a fatal failure: best-effort thumbnail/cache cleanup and post-commit cleanup can remain nonfatal, with a deliberate policy and diagnostics.

**Target dependency direction**

```text
Tauri setup, tray, window lifecycle
                  |
          TauRPC feature adapters
                  |
       Feature application services
              /               \
  Domain models and         Explicit effects
  pure decisions                 |
                    Filesystem / HTTP / keyring /
                    process adapters
```

Services receive the small dependencies they need. Pure decisions receive values and return values. Neither imports `crate::ipc`, generated event triggers, or a Tauri runtime. Tauri's `AppHandle` stays in application adapters except where an actual native API requires it.

Create an application-owned state object for shared HTTP clients, repositories, process managers, supervisor, tunnel manager, and lifecycle state. Resolve application directories into `AppPaths` once during setup. Use Tauri managed state in resolvers and clone `Arc` handles for long-running jobs. Avoid replacing several globals with one giant mutex or passing all application state into every function.

Preserve separate HTTP clients where cookies, authentication, redirect policies, or timeouts differ. Ely web sessions should not share an unrelated global cookie jar. Inject provider base URLs for local test servers. Extract shared provider request/response code only when content, plugins, and version changes truly use the same semantics.

Use traits selectively for effects that need substitution: credential storage, process control, provider lookups, and transaction operations with injected failures. The existing `keyring_store::SecretStore` and fake store are a good local model. Plain structs, paths, functions, and callbacks are sufficient elsewhere. Pass timestamps to pure rules; use a fake clock only for time-driven workflows.

Repositories should serialize read-modify-write operations per store, so simultaneous commands cannot overwrite unrelated updates. Atomic file replacement protects a single write; it does not by itself prevent lost updates. Keep potentially long HTTP operations outside repository locks and revalidate state before committing their results.

For version changes, persist transaction identity, previous metadata, intended file operations, and phase information before destructive steps; record enough progress to recover interrupted operations. Keep the previous usable backup until the replacement transaction commits. Treat multi-file changes as a recoverable protocol, not a claim that a sequence of filesystem operations is atomic. Application startup must detect incomplete transactions and expose an explicit recovery outcome before allowing a conflicting launch/change.

Move heavy synchronous archive work, hashing, NBT/image processing, directory scans, and copy operations onto an explicit blocking boundary. Preserve the existing `spawn_blocking` use around map and transfer operations. Handle task join failures distinctly from domain failures. Keep lock scopes short and define ownership/cancellation for background tasks.

**TauRPC decomposition and migration**

Use the path-based approach in the request. It is supported by the installed version, including nested paths. Proposed ownership:

| Path | Responsibility |
| --- | --- |
| `app` | Quit blockers, shutdown, leftover-process decisions |
| `accounts`, `skins` | Accounts/auth flows; skin catalog, apply/upload/download |
| `settings` | Memory, window, launcher behavior, global sync preferences |
| `instances` | Instance CRUD, launch/kill, running instances, worlds |
| `instances.sync`, `instances.imports`, `instances.screenshots` | Their respective feature workflows |
| `versions` | Available Minecraft, loader, and server-core versions |
| `content` | Search, details, file installation, modpack installation |
| `bedrock` | Native client discovery and launch/install/open operations |
| `servers` | Server CRUD and lifecycle |
| `servers.config`, `servers.players` | Editable properties/files/access lists; player information |
| `servers.plugins`, `servers.packs` | Java plugins/mods; Bedrock packs |
| `servers.map`, `servers.metrics` | World maps; statistics, storage, and network information |
| `servers.transfer`, `servers.tunnel` | Import/export/duplicate and chunk uploads; Playit |
| `instances.version_change`, `servers.version_change` | Feature-specific adapters over one version-change service |
| `events` | The existing eight typed events, in one small event-only API |

Each leaf feature contains its procedures trait and thin resolver together until size warrants separate files. `ipc/mod.rs` only assembles the router. Initially keep method names, argument names/order, and DTO shapes unchanged; adding paths is already a contract change.

Conceptually, router composition becomes:

```rust
taurpc::Router::<tauri::Wry>::new()
    .merge(AppApiImpl.into_handler())
    .merge(AccountsApiImpl.into_handler())
    .merge(SettingsApiImpl.into_handler())
    .merge(InstancesApiImpl.into_handler())
    .merge(ServerPluginsApiImpl.into_handler())
    .merge(EventsApiImpl.into_handler())
// Remaining feature handlers follow the same pattern.
```

This is an architectural sketch, not compiled implementation. State injection and concrete constructors are settled in the pilot.

Migration rules:

1. Record every current command/event and its destination. Check for omissions and duplicate paths. In TauRPC 0.8.2, merging a duplicate path replaces map entries; it is not a mechanism for combining two traits at the same path.
2. Move one namespace and its frontend callers together. For example, `rpc.get_accounts()` becomes `rpc.accounts.get_accounts()`. Update service modules, direct component calls, event subscriptions, and backend event emitters.
3. Retain one temporary root compatibility trait only if needed for staged changes. It delegates to the same services and has a removal milestone. Do not keep duplicate business logic or emit every event twice.
4. Add a shared frontend proxy module instead of constructing it in each service. Include direct users such as `bedrock-packs-panel.tsx` in the migration.
5. Export bindings with a dedicated command/test that does not launch the UI or run setup side effects. Router construction needs an async runtime because handlers spawn tasks. Fail export errors; the current startup code discards them.
6. Generate into a temporary file in CI and compare with committed `src/bindings.ts`; run `pnpm exec tsc --noEmit -p .`. Never hand-edit generated bindings.
7. Test at least one nested procedure and event end to end through the Tauri boundary. TypeScript compilation cannot detect an event whose runtime path no longer matches its listener.

[TauRPC routing and custom-error documentation](https://docs.rs/crate/taurpc/latest) supports this design. The installed Rust crate and npm runtime were also inspected.

**Error handling and type safety**

Use `thiserror` for errors that code must distinguish. Give features small error enums such as `AccountError`, `LaunchError`, and `VersionChangeError`, supported by shared storage/download errors. Preserve source errors with `#[source]` or `#[from]`; attach operation/path/provider context where useful. Avoid a catch-all `Message(String)` variant becoming the replacement for every existing string.

Keep internal errors separate from the wire contract. Native I/O and HTTP errors should not be serialized directly. Map them explicitly to a compact `ApiError` enum deriving `serde::Serialize` and `specta::Type`, with a stable discriminator and typed details. Initial categories should cover invalid input, missing resource, conflicting state, authentication required, cancellation, unavailable provider, storage failure, incomplete recovery, and an internal failure with a diagnostic reference. Local logs retain the source chain; frontend messages exclude credentials and raw provider bodies.

Use `anyhow` only where heterogeneous failures are reported rather than matched, such as startup or a bindings-export tool. It is optional. Replacing every string with `anyhow::Error` would improve diagnostics but would not create an exhaustive domain error model. See the primary [thiserror](https://docs.rs/thiserror/latest/thiserror/) and [anyhow](https://docs.rs/anyhow/latest/anyhow/) documentation.

For typed frontend errors, eventually configure:

```rust
taurpc::Exporter::new()
    .error_handling(taurpc::ErrorHandlingMode::Result)
    .export(&router, output_path)?;
```

A fallible procedure then generates a result union with `status: "ok"`/`data` or `status: "error"`/`error`. Consumers can narrow on the status and the API error discriminator. This exporter switch is global: perform it as a coordinated frontend migration after the namespaces and error schemas stabilize. Do not silently mix old `Promise<T>` consumers with result envelopes. Existing React Query and toast handling need explicit adaptation so an error envelope is not treated as successful data. Avoid immediately throwing all typed errors away in a generic unwrap helper.

The installed JS runtime still rethrows JavaScript `Error` objects and wraps other rejections. Keep an `unknown` fallback at the transport boundary and validate unexpected rejection shapes before treating them as `ApiError`. Generated types are a contract, not runtime validation of arbitrary failures. See [TauRPC error handling](https://docs.rs/crate/taurpc/latest).

Strengthen the data model alongside each feature:

| Current shape | Proposed shape / invariant |
| --- | --- |
| `AccountProfile.account_type: String` | `AccountKind::{Ely, Microsoft, Offline}` with existing JSON spellings |
| Strings for behavior, content source/type, sync category, phase, and log level | Feature enums; keep free-form detail text separate from machine-readable state |
| All IDs as interchangeable strings | Private-field `AccountId`, `InstanceId`, `ServerId`, `UploadId`; validated construction and deserialization |
| Account IDs treated like arbitrary UUIDs | Preserve actual `ely:...`, `microsoft:...`, and `offline:...` formats and existing keyring identifiers |
| Independent RAM numbers and optional loader fields | Validated memory limits and internal loader selection that requires a version for modded loaders |
| `QuickPlayOptions` with two optional targets | Internal `QuickPlayTarget::{Server, World}` so conflicting selections are rejected |
| A target kind plus unrelated string ID | Internal `VersionChangeTarget::{Instance(InstanceId), Server(ServerId)}` |
| Manifest argument value as general `serde_json::Value` | An untagged string-or-string-list type at deserialization |
| Filenames, relative paths, and download URLs as unchecked strings | Validated types with explicit containment, scheme/host, and checksum policies |

Reuse existing useful enums: `ModLoaderType`, `ServerCoreType`, `ServerStatus`, `AccessListKind`, and version-change action/status enums. Keep Minecraft version identifiers flexible enough for snapshots and non-semver release names. Do not impose semantic-version parsing on all version strings.

Preserve existing persisted JSON shapes through conversion DTOs, serde names/defaults, and compatibility fixtures. Do not rewrite all stored data as a side effect of reorganizing modules. A transparent Rust ID wrapper may still export as a TypeScript string; it protects Rust call sites but does not automatically give nominal TypeScript IDs. Audit large integer fields at the IPC boundary for JavaScript safe-integer limits.

These changes enforce the invariants represented by the types. They cannot guarantee absence of runtime failures, filesystem races, provider changes, or every logical bug.

**Test design**

Default tests must use no public network services, real accounts, installed Minecraft/Java, or unlocked native vault. Separate pure unit tests, local adapter integration tests, and explicit platform/live smoke tests. Move the five current live API tests behind explicit opt-in execution and replace their default role with response fixtures and local HTTP tests.

| Area | Required tests |
| --- | --- |
| Settings and persistence | Defaults for missing files; malformed/unreadable input; old JSON compatibility; validation; failed replacement preserves previous file; concurrent updates do not lose unrelated changes |
| Accounts/auth | One active account; selection/removal/reorder rules; provider-specific account kind; token-expiry margin; refresh failure; cancellation; device-code pending/slow-down/expiry/decline; fake credential store; concurrent refresh behavior |
| Launcher | Exact argument vectors for vanilla/Fabric/Quilt/Forge/NeoForge; OS rule evaluation; classpath; RAM/window inheritance; quick play; missing assets/Java; start reservation; exit/playtime/sync decisions |
| Importers | Small fixtures for each supported launcher format; incomplete metadata; loader/version parsing; include/exclude copy rules; required and optional failures; aborted import cleanup |
| Providers/content/plugins | Query mapping, pagination, compatible/stable release selection, malformed responses, 401/403/404/429/5xx, partial search, dependency cycles/conflicts, missing download URL, old installation retained on failure |
| Downloads/archives | Checksum and size rules, range boundaries and fallback, wrong/short range response, cancellation, traversal and Windows path forms, link handling, destination containment |
| Version changes | Compatibility matrix, downgrade decisions, stale plans, dependency resolution, checksum failure, failed file swap, rollback failure, journal failure, config commit failure, undo, restart recovery |
| Server lifecycle | Concurrent starts, cancelled startup, readiness, stop timeout, process exit, stale generation/PID protection, sleep/wake, shutdown during startup, late/cancelled console query |
| Live data/maps | SNBT malformed/truncated input, escaped strings, item/player format variants, NBT fixtures, Java and Bedrock chunks, palette/raster output, cache invalidation and rendering options |
| Sync/screenshots | Conflict decisions using explicit timestamps; failed backup/copy reports; effective overrides; path containment; invalid images; thumbnail invalidation |
| IPC | DTO serialization and compatibility; domain-to-API error mapping; router path inventory; binding freshness; typed success/failure consumers; event route and listener cleanup |

Use `tempfile` for isolated filesystem fixtures. Add one HTTP mock library, not several. Add `tokio`'s `test-util` dev feature for paused-time tests; `full` alone should not be assumed to provide it. Use property tests selectively for parsing, paths, VarInts, and serialization round trips. Check image pixels or decoded PNGs rather than relying solely on compressed-byte snapshots.

Preserve and extend the existing strong tests for credential chunking/failure recovery, properties escaping, offline UUIDs, plugin metadata, and version-change swap/undo. Keep tests beside the code when small; put larger suites in feature-local test modules and shared fixtures in `src-tauri/tests/fixtures`.

Record coverage with `cargo-llvm-cov` before setting a repository-wide gate. Use a no-regression ratchet, with proposed completion targets of at least 80% line coverage for refactored feature logic and 90% for extracted pure decision/validation modules. Require the failure scenarios above regardless of percentage. Report and justify excluded generated/native glue explicitly; do not exclude difficult business logic. Coverage is evidence of execution, not proof of correctness.

**Implementation sequence and acceptance gates**

1. **Establish a reliable baseline.** Inventory the 140 commands and 8 events, store representative legacy JSON and provider fixtures, isolate live tests, and run the complete deterministic suite on Windows/Linux/macOS. Record formatting, lint, and coverage baselines. Keep any bulk formatting separate from logic changes. Gate: ordinary test runs require no public network or credentials.

2. **Build one complete pilot using settings.** Extract settings validation and persistence from `system.rs`; introduce `AppPaths`, storage errors, atomic replacement, and a small application state entry. Keep the existing RPC contract while proving dependency injection and temporary-directory tests. Gate: defaults, validation, corruption, and failed writes are tested without Tauri setup. Expand shared helpers only after this slice works.

3. **Split IPC and remove reverse dependencies.** Add feature namespaces, application lifecycle/event adapters, and managed process state. Migrate frontend callers one feature at a time, initially retaining the current error/DTO behavior. Move lifecycle and version-change orchestration out of transport handlers. Gate: every command/event accounted for, no feature-to-IPC imports, fresh bindings, TypeScript check, nested route/event smoke test, temporary root bridge removed.

4. **Address failure integrity in focused changes.** Characterize and then fix corrupt account loading, archive containment, incomplete imports, unverified cached downloads, unsafe plugin replacement, and version-change recovery. Extract only the boundaries needed for these tests before larger moves. Use typed internal errors; an outer string mapping can temporarily preserve RPC compatibility. Gate: injected failures preserve data/recovery material and never claim complete success incorrectly.

5. **Refactor feature internals in small slices.** Suggested order: accounts/auth; importer and content/providers; launcher; server configuration/process/supervisor; live data/maps; remaining sync/screenshots/platform helpers. For each slice, extract pure decisions, add unit tests, move effects behind adapters, and replace internal string errors. Preserve Android/desktop behavior and existing storage formats. Gate: targeted tests, compatibility fixtures, and relevant platform checks pass after each slice. No single change should combine the entire launcher and server lifecycle rewrites.

6. **Finish the typed IPC contract.** Finalize serializable API errors and validated DTOs, enable TauRPC result mode once, regenerate bindings, and update service/UI result handling together. Add compile-time TypeScript cases for exhaustive expected-error handling and runtime tests for unexpected rejection shapes. Gate: no string errors in application-owned RPC signatures; callers distinguish expected failures from transport failures without unsafe casts.

7. **Enforce maintainability and platform checks.** Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, deterministic tests, binding comparison, TypeScript checks, and the coverage ratchet. Resolve existing lint debt in scoped changes before requiring a clean gate. Maintain native keyring tests separately and add an Android target compile job with the necessary NDK/JNI setup. Gate: documented responsibilities, reviewed large-file exceptions, and green supported-platform checks.

Preserve unrelated dependency pins and the current TauRPC/Specta pairing unless a demonstrated compatibility issue requires an upgrade. Update Cargo.lock for required dependency additions such as typed-error or test support. Do not combine this refactor with framework upgrades, a database migration, companion-JAR rewrites, or new launcher features. Native smoke tests remain necessary even with high unit coverage.

**Execution checkpoints for agents**

Use a short task record for each implementation slice. Resolve routine implementation choices without another approval cycle; the record is a work/handoff aid. Do not treat this roadmap as authorization to implement work the user has only asked to plan.

```text
Phase and slice:
Scope / files owned:
Behavior to preserve:
Intentional behavior change, if any:
Contracts / persisted formats affected:
Acceptance criteria and regression cases:
Checks run and actual results:
Remaining adapters, unverified checks, or known limitations:
Next concrete step:
```

If multiple agents are explicitly assigned, identify one integration owner for shared types, the router, dependency manifests, and bindings. Each slice must build and test against agreed interfaces; file moves and frontend contract migrations need coordination before integration.

Current checkpoints (update after implementation, retaining the historical baseline above):

Implementation evidence and handoff: [src-tauri/REFACTOR_PROGRESS.md](src-tauri/REFACTOR_PROGRESS.md).

| Work | Status | Evidence / exit condition |
| --- | --- | --- |
| Analysis and initial test baseline | Complete | 52 passed; 6 ignored; 5 live tests filtered; Windows only |
| Coding and agent guidance | Complete | Root and Rust AGENTS.md files; plan linked to rules |
| Phase 1: deterministic suite and contract inventory | Windows verified; desktop CI pending | Live/provider/vault/installed-app smoke tests are opt-in; current inventory has 155 procedures (147 commands, 8 events); provider and stored-format fixtures checked; Windows coverage measured; Linux/macOS execution remains unverified locally |
| Phase 2: settings pilot | Implemented; Windows verified | 11 settings tests; typed errors; application-owned store; legacy/current JSON compatibility; remaining platform/coverage evidence tracked below |
| Phase 3: IPC and state separation | Namespaces migrated; remaining adapters tracked | Router/binding inventory accounts for all namespaces; launcher preparation, imports, sync/screenshots, version mutations and server lifecycle/supervision are testable without Tauri setup. Native server command/event adapters live in app; content installation/search now use explicit services and application adapters; screenshot file-manager effects are consolidated. Instance, transfer, and sandbox feature operations now use repositories/paths and application-owned services; Tauri wiring resides in app adapters |
| Phase 4: failure integrity | Implemented slices verified; broader audit remains | Restart deletion/directory-publication receipts, plugin batch commit/recovery, generation ownership, verified downloads and retryable version restoration covered by failure regressions; no universal multi-resource/power-loss atomicity claim |
| Phase 5: feature extraction | Five corrective assignments implemented; broader audit remains | Native lifecycle adapters, bounded Java/installer execution, owned Playit generations, explicit scan/journal failures and publication protocols have focused regressions. The six transaction/content/platform audit findings are fixed: typed transaction causes, retryable cleanup, write-ahead disable intent, explicit provider failures and independent content services. Instance services, transfer maintenance/upload ownership, and sandbox setup/publication recovery are also extracted and tested. Launcher, screenshot and importer instance-error wrappers now preserve typed causes and delegate API categories; other effect boundaries remain subject to audit, and file size alone does not close the phase |
| Phase 6: typed IPC errors | Complete | RPC signatures use ApiError/TauRpcResult; migrated services map typed feature errors directly; ipc/error/legacy.rs eliminated and removed; zero String-to-ApiError fallback remains; frontend result/transport/failure/stale-response cases covered |
| Phase 7: enforcement | Windows/Android gates verified; desktop CI and coverage targets pending | 689 Rust tests passed, 17 ignored (including 4 subprocess fixtures exercised by parent tests); fmt/Clippy/bindings/typecheck/Biome/14 IPC tests passed; Android compile and Windows vault passed. Windows measured line coverage 57.82%, minimum raised to 57.81%; Linux/macOS execution and 80% feature / 90% pure-rule targets remain open |

**Completion criteria**

- Feature code can be understood and tested without reading a monolithic RPC implementation.
- No application logic remains in a 1,000-line function/file by default; legitimate tables or fixtures have explicit exceptions.
- The default suite is deterministic, covers the listed failure cases, and runs across desktop platforms.
- Tauri adapters are thin, process/lifecycle state has clear ownership, and domain code does not import IPC.
- Internal errors retain useful sources; expected frontend errors are typed and deliberately mapped.
- Existing account IDs, keyring data, instances, servers, settings, and backup formats remain compatible or have explicit tested migrations.
- Failed imports, updates, and recovery operations leave an honest result and a recoverable state.
- Generated bindings and all command/event consumers agree in CI.

The first implementation change should establish deterministic tests and the contract inventory. The first architectural slice should be settings. Start the broader refactor only after that slice demonstrates the intended pattern.
