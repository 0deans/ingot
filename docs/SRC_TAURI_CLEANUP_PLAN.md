# src-tauri cleanup and architecture plan

## Scope and instructions

- Scope: `src-tauri` only. No frontend, generated Android, vendor, bundled binary, or unrelated CI changes.
- This document plans implementation; creating it does not authorize starting implementation. When assigned to execute it, the orchestrator may delegate the bounded tasks below.
- Read root `AGENTS.md`, `src-tauri/AGENTS.md`, and relevant sections of `docs/RUST_REFACTOR_PLAN.md`. Preserve the historical baseline in the existing plan.
- Preserve IPC contracts, stored formats, platform behavior, credential recovery, and cancellation guarantees.
- Reduce duplicated behavior and unnecessary dependencies. Line count and crate count are not success criteria by themselves.
- Findings are based on source inspection, not an exhaustive duplication scan or a verified test baseline. Revalidate against current code before editing.
- Do not stage, unstage, or commit without explicit user authorization.

## Orchestrator workflow

1. Inspect working-tree changes and establish the baseline in P01.
2. Assign each agent an explicit task ID, exclusive file scope, prerequisites, and acceptance criteria.
3. Keep one integration owner for manifests, Cargo.lock, crate exports, shared interfaces, router composition, and this checklist. Agents propose changes to those shared files to that owner.
4. Agree on interfaces before parallel implementation. Do not let agents independently invent competing shared helpers.
5. Separate mechanical moves, behavioral fixes, and dependency changes into reviewable slices. Keep intermediate states compiling.
6. Integrate and verify each slice before starting its dependent tasks. Serialize workspace-wide checks and generated-file operations.
7. Record completed work, checks, remaining adapters, and justified no-change decisions in the progress table. A candidate may be rejected with evidence; do not create abstractions simply to check a box.
8. If a change requires modifying IPC contracts or files outside scope, isolate that proposal for a separate scope decision. Continue independent in-scope work.

### Parallel work schedule

| Wave | Tasks that may run concurrently | Coordination |
| --- | --- | --- |
| 0 | P01 baseline | Orchestrator owns inventory and verification baseline |
| 1 | P02 preparation tests; P03 HTTP fixture pilot; P05 filesystem policy inventory | P03 excludes P02 files; P05 is initially read-only |
| 2 | P04 compatibility cleanup by crate; P06 map separation | Do not assign overlapping crates; integration owner applies shared exports/manifests |
| 3 | P05 filesystem implementation; P07 launcher/account boundary; selected P08 reviews | Agree shared contracts first; exclude overlapping owners |
| 4 | P09 justified extractions; P10 app/IPC cleanup | Sequence each extraction before its affected adapters |
| 5 | P11 dependency/build audit; P12 final verification | Integrate first; avoid concurrent manifest or workspace-format mutations |

Use fewer agents when scopes overlap. Do not run all tasks at once.

## P01 — Establish the baseline

- Inventory crate dependency edges, public exports, and forwarding wrappers.
- Separate implementation, test, fixture, generated, and lookup-table line counts.
- Identify repeated implementations, distinguishing matching syntax from matching semantics.
- Record current tests/lint failures and available platform targets.
- Compare current code with the existing refactor roadmap; do not treat historical defects as current without checking.

**Acceptance:** each candidate has concrete locations, callers, ownership, risks, and verification requirements. No unsupported claim of complete coverage or exhaustive duplication detection.

## P02 — Simplify local tests

Start: `crates/minecraft/src/launcher/prepare/tests.rs`.

- Add a small local fixture for cached versions, asset indexes, and preparation paths.
- Consolidate repeated `FakePreparation` construction.
- Keep scenario inputs and expected outcomes visible.
- Preserve Java/authentication failure, invalid hash, no-network, and deduplication scenarios.
- Keep call-count assertions when they express required behavior, such as authentication not running after Java failure.
- Review large content, version-change, and account suites next; organize by scenario responsibility.
- Keep private tests local. A directory containing only `tests.rs` is not itself a problem.

**Acceptance:** less repeated setup, preserved meaningful scenarios, no public API expansion solely for testing.

## P03 — Consolidate HTTP test infrastructure

Candidates: artifacts source/download tests, accounts protocol tests, content installation tests, version-change staging tests.

- Pilot a shared loopback fixture with two representative consumers.
- Support ephemeral ports, scripted responses, request capture, and explicit completion checks.
- Surface task failures; clean up sockets/tasks on drop and cancellation.
- Support controlled delay/interruption only where actual scenarios require it.
- Keep provider payloads, credential fakes, and domain assertions local.
- Introduce a development-only support crate only if the pilot demonstrates cross-crate value.

**Acceptance:** duplicated transport code is removed without hiding request assertions, losing failure control, or depending on public network access.

## P04 — Remove migration scaffolding

Scope: root and crate `lib.rs` files and their consumers.

- Replace transitional aliases with direct imports.
- Remove synthetic namespaces that obscure ownership, including content's combined Minecraft namespace.
- Review wildcard exports and expose deliberate APIs.
- Remove compatibility wrappers only after checking callers.
- Review paired APIs such as `apply`/`apply_typed` and `check`/`check_typed`; retain one implementation plus only necessary boundary adaptation.

**Acceptance:** imports identify actual owners; unnecessary wrappers/exports disappear; IPC and required public compatibility remain intact.

## P05 — Consolidate filesystem primitives

Compare Minecraft duplication/sync, content importing, server transfer, and version-change transaction paths.

Before editing, build a policy matrix for:

- Symlinks, Windows junctions, and special entries.
- Top-level versus recursive exclusions.
- Existing destinations and atomic replacement.
- Cancellation, rollback, and restart recovery.
- Tracking/metadata preservation and error context.

Extract only shared mechanisms such as validated traversal or staged replacement. Keep workflow-specific policy local. Avoid one generic copy function controlled by many booleans.

**Acceptance:** shared mechanisms have one implementation; platform and failure-path tests preserve each workflow's guarantees. No recovery material is discarded before completion is acknowledged.

## P06 — Separate map rendering from server control

Evidence: `crates/server-map/src/live/query.rs` uses `ServerProcessManager`; live errors contain server process errors.

- Keep world reading, parsing, rasterization, and tile caching in the rendering domain.
- Move live-query orchestration toward its server/application owner.
- Keep reusable parsing separate from process effects.
- Keep process errors out of pure rendering contracts.
- Add a narrow query capability only where real substitution is needed.
- Check prospective dependency direction before moving live modules; avoid creating a cycle.

**Acceptance:** pure map functionality builds and tests without requiring the server supervisor dependency; live query behavior is preserved.

## P07 — Narrow launcher/account coupling

Evidence: launcher models reference account `RecoveryError` and `SessionError`; preparation already exposes `PreparationEffects`.

- Review authentication adaptation at the application boundary.
- Keep session, persistence, and recovery ownership in accounts.
- Let the launcher consume resolved authentication through a focused contract.
- Preserve useful typed error categories and causes; do not replace them with strings to remove an edge.
- Preserve token checkpointing and cancellation/recovery ordering.

**Acceptance:** launcher behavior does not require account persistence/recovery internals, and failure diagnostics remain actionable. Remove the dependency only if the resulting contract is genuinely simpler.

## P08 — Review internal responsibility boundaries

These are investigation targets, not mandatory file splits.

| Area | Responsibility review |
| --- | --- |
| Minecraft loader | Provider/version resolution, profile construction, execution |
| Horizon preparation | Planning, artifact preparation, platform execution |
| Content | Discovery/provider conversion, installation, instance publication |
| Server installation | Installer, downloader, Bedrock, Pumpkin ownership |
| Accounts | Remote protocols versus sessions, credentials, recovery |
| Version changes | Planning, staging, commit, recovery under one workflow owner |
| Artifacts | Whether crash/journal and target models belong to acquisition or version changes |
| Core | Shared primitives only; avoid accumulating unrelated feature policy |

**Acceptance:** extracted components own coherent behavior; parents coordinate fewer internal steps. Larger cohesive modules may remain with a recorded reason.

## P09 — Evaluate crate extractions

| Candidate | Prerequisite | Direction |
| --- | --- | --- |
| Sandbox | Isolated platform execution contracts/assets | Strong candidate after boundary review |
| Tunnel | Independent status and execution contracts | Remove server configuration/sandbox coupling first |
| Server installer | Defined installation inputs/results and execution effects | Start as an internal feature |
| Remote authentication | Stable protocol/session/recovery contracts | Defer until token lifecycle boundaries are clear |
| Instance domain | Evidence content needs only narrow instance contracts | Extract only if substantial coupling is removed |

For every extraction:

- Keep the graph acyclic and prevent dependency back to the former owner.
- Put contracts with their semantic owner; avoid a catch-all shared-types crate.
- Require independent testing, platform isolation, meaningful dependency reduction, or real reuse.
- Preserve platform branches, assets, stored formats, and dependency pins unless a deliberate change is required.

**Acceptance:** reduced coupling demonstrated by before/after dependency edges and simpler callers. A justified decision not to extract is valid completion.

## P10 — Simplify application and IPC layers

Scope: `src/app` and `src/ipc`, after affected domain changes stabilize.

- IPC owns input boundaries, service calls, and error mapping.
- App owns wiring, native adapters, and cross-feature coordination.
- Move domain workflows to their feature owner.
- Remove forwarding layers that provide neither policy nor adaptation.
- Keep one authoritative owner for repositories, managers, and lifecycle state.
- Preserve unique TauRPC paths and single router registration.

**Acceptance:** behavior is not duplicated between app and IPC; domain services do not import Tauri/IPC implementation details; contracts remain unchanged.

## P11 — Audit dependencies and build cost

- Verify direct dependency usage, including tests, macros, and platform paths.
- Remove dependencies left over from extraction.
- Review broad features against actual requirements.
- Evaluate Bedrock gating across the whole graph: the root also directly declares `bedrock-world`.
- Measure clean builds, incremental builds, and focused tests before changing compiler profiles.

**Acceptance:** dependency removals are verified on applicable targets; build improvements are measured rather than inferred from crate count.

## P12 — Verification and completion

For each implementation slice:

1. Add a regression before fixing a known defect.
2. Run focused tests first.
3. Run applicable repository-required checks from `src-tauri`:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

4. Follow all additional checks required by `src-tauri/AGENTS.md`. Where it requires frontend compatibility checks, running them is validation, not authorization to edit frontend files.
5. Verify affected platform behavior; report unavailable targets honestly.
6. Keep IPC unchanged. If a contract change becomes necessary, report the separate scope requirement; never hand-edit generated bindings or silently omit required consumer/inventory updates.
7. Inspect the final diff and working tree. Preserve unrelated changes.

Measure completed work by duplicate implementations removed, dependency edges eliminated, public APIs narrowed, fewer edit locations per feature, preserved failure coverage, and measured build/test changes.

### Agent handoff format

- Task ID and owned files.
- Changes and rationale.
- Behavior/contracts preserved or deliberately changed.
- Tests/checks run with results; unrun checks and reasons.
- Remaining adapters, risks, and dependencies on other tasks.
- Proposed shared-file changes for the integration owner.

### Progress

| Task | Status | Evidence / remaining work |
| --- | --- | --- |
| P01 Baseline | Done | Recorded workspace baseline checks and test counts (`ipc_contract`, `tsc`, `pnpm test:ipc`, workspace suite) |
| P02 Local tests | Done | Kept tests local/loopback with hermetic fakes; simplified `crates/minecraft/src/launcher/prepare/tests.rs` and split monolithic test suites in `crates/version-change/src/tests.rs` (`swap_tests`, `recovery_tests`, `symlink_tests`) and `crates/content/src/content/` (`third_party_tests`, `install/pack_third_party_tests`) |
| P03 HTTP test support | Done | Extracted dev-only `ingot-test-support` workspace crate (`crates/test-support`: `LoopbackServer`, `ScriptedExchange`, `ResponseSpec`, `CapturedRequest`, `RequestEffect`) and migrated HTTP loopback suites in `accounts`, `content`, `minecraft`, `sandbox`, `server`, and `version-change` (keeping `artifacts` downloader/source socket helpers local) |
| P04 Migration scaffolding | Done | Removed synthetic `ingot_content::minecraft` and unused `ingot-settings` dep; removed all remaining `#[deprecated]` items/wrappers across `src` and `crates` |
| P05 Filesystem primitives | Done | Extracted `is_link_or_reparse_point`, `persist_staged_file`, atomic write/copy helpers in `crates/core/src/fs.rs` and adopted across crates |
| P06 Map/server boundary | Done | Decoupled `ingot-server-map` from `ingot-server` and moved live-query orchestration into `crates/server/src/live_query.rs` |
| P07 Launcher/accounts boundary | Done | Removed `ingot-accounts` dep from `ingot-minecraft`; introduced `LaunchAccountError` with typed downcasting at IPC boundary |
| P08 Internal responsibilities | Done | Refactored `crates/minecraft/src/loader.rs` + `loader/tests.rs` and split `horizon/prepare.rs` into `artifacts.rs`, `runner.rs`, and `tests.rs`; reviewed remaining owners (`content/install/pack.rs` delegates to `ftb_pack`/`technic_pack`/`atlauncher_pack`, `server/installer.rs` + `execution.rs`, `accounts` auth/session/recovery modules, `version-change` transaction modules, `ingot-artifacts` shared `crash`/`targets`, and domain-neutral `ingot-core`) |
| P09 Crate decisions | Done | Centralized `USER_AGENT` in `ingot-core`; extracted `ingot-sandbox` and `ingot-tunnel` (with `PlayitTunnelStatus`); retained `installer`, `auth`, `instance` in owners |
| P10 App/IPC ownership | Done | Removed `app/news.rs` and `app/mod.rs` forwarding glue; reused `app::platform::open_directory` for servers; deduplicated `is_downgrade` in `ipc::versions`; narrowed root `src/lib.rs` `accounts`/`content`/`server-map` re-exports and removed unused root aliases |
| P11 Dependencies/builds | Done | Removed unused direct root/`content`/`minecraft`/`server-map` deps (incl. root `bedrock-world` and `server-map` `tokio`/`ingot-core`); scoped `reqwest` `cookies`/`multipart` to `accounts`, narrowed workspace `tokio` features, and scoped `ingot-test-support` to `[dev-dependencies]`; measured cold clean check (`355.59s`), cold test build+suite (`312.22s`), incremental checks (`ingot`: `16.14s`, `ingot-core`->workspace: `11.11s`), and warm focused tests (`ingot-version-change`: `1.53s`, `ingot-accounts`: `3.69s`); Windows cross-target checks for `x86_64-unknown-linux-musl` and `aarch64-linux-android` blocked in `ring v0.17.14` by missing host C cross-compilers |
| P12 Final verification | Done | Passed `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked` (932 passed, 0 failed, 19 ignored), `ipc_contract::generated_bindings_match_the_committed_contract`, `pnpm exec tsc --noEmit -p .`, and `pnpm test:ipc` |
