**Rust backend coding and refactor rules**

These rules apply to work under `src-tauri`. For backend-driven frontend/CI changes, also follow the contract and verification rules below. Read the relevant phase of `../RUST_REFACTOR_PLAN.md` before refactoring. Existing code is being migrated incrementally: apply these rules to new and substantially refactored code, and record remaining migration work rather than rewriting unrelated modules.

The current user request determines scope. Resolve routine design choices using these rules and the code; record material tradeoffs with the change. This file does not require additional approval for ordinary refactoring already requested by the user.

**1. Keep each change understandable**

- Choose a bounded feature or responsibility with observable acceptance criteria. Read its callers, tests, persisted formats, and platform branches before changing it.
- Separate mechanical moves/renames, behavior fixes, and broad formatting into reviewable changes. Keep intermediate changes compiling. Do not combine a framework upgrade with a structural refactor.
- Preserve existing behavior during extraction. For a known defect, write a regression that expresses the desired behavior and fix it explicitly; do not silently make the defect a permanent specification.
- Use the plan's destinations as guidance, not a requirement to create empty files or unused abstractions. Update the plan when a justified design decision changes a destination or migration order.
- If work has been explicitly divided among agents, give each a non-overlapping scope and designate an owner for shared contracts, router composition, manifests, and bindings. Coordinate interface changes before integration; each agent reports its tests and remaining work.

**2. Organize by responsibility**

- Keep related models, rules, effects, and tests in their feature directory. Avoid generic dumping grounds such as `utils.rs`, a global `types.rs`, or a giant shared error enum.
- Aim for implementation files around 150–350 lines. Review files above roughly 500 lines and functions above roughly 60 lines for mixed responsibilities. These are review triggers, not automatic failures; record a reason for a cohesive larger unit. Count implementation separately from test fixtures, generated code, and lookup tables.
- Keep `mod.rs` focused on module declarations and intentional exports. Prefer private items; use `pub(super)` or `pub(crate)` when needed. Do not make internals public merely to test them.
- Name functions for the work they do. `load` and `find` should not create directories; use explicit names such as `ensure_instance_dir` for creation. Keep parsing, validation, planning, execution, and persistence distinguishable.
- Prefer small concrete functions and structs. Add a trait when an actual effect needs substitution or multiple implementations; do not build a generic repository/service framework for hypothetical reuse.
- Use clear control flow, early returns, and exhaustive matches. Avoid clever iterator chains, deep nesting, boolean mode arguments, and anonymous tuples that hide meaning. Use a named request/options/result type when a signature becomes hard to follow.
- Follow rustfmt. Comments explain an invariant, protocol detail, or non-obvious decision. Document module responsibilities and public contracts; avoid comments that narrate straightforward code and placeholder TODOs without a concrete follow-up.

**3. Keep dependencies directed toward the feature**

- RPC handlers deserialize/validate boundary input, call a feature service, and map the result. They do not implement downloads, filesystem transactions, compatibility rules, or process lifecycle policy.
- Pure rules accept values and return values. They do not access Tauri, files, the network, the clock, environment variables, or global state. Pass required context as values.
- Feature services orchestrate pure rules and explicit effect adapters. They must not import `crate::ipc`, generated TauRPC event triggers, or window/tray APIs.
- Keep Tauri handles and runtime types at application/native adapters. Emit domain events through a callback or small interface; the app layer maps those events to TauRPC or native UI actions.
- Own shared clients, repositories, process managers, and lifecycle state in application state. Pass each service only what it needs. Do not add mutable global singletons or put all application state behind one mutex.
- App setup registers `AppState` before accepting commands or starting background work. Its construction can fail when configuring the shared HTTP client; propagate that failure. Native adapters can use the managed-state accessor under this wiring invariant. Pass or clone specific manager/client handles for work that outlives a caller; do not require a global `'static` manager.
- Share HTTP clients when their configuration permits it. Keep credential/cookie sessions isolated and preserve provider-specific redirect, authentication, and timeout behavior. Make endpoints injectable for tests.

**4. Make valid states explicit**

- Use enums for finite categories and states. Reuse the existing loader/core/status/action enums; keep display text separate from machine-readable values.
- Introduce private-field ID/value types where confusion or invalid values can cause incorrect behavior. Validate construction and deserialization, not just one UI caller. A `String` wrapper without validation does not establish validity.
- Use `Path`/`PathBuf` internally, `Duration` for elapsed intervals, and names that identify numeric units. Borrow data when ownership is unnecessary; clone at clear ownership/task boundaries without introducing needless lifetime complexity.
- Use `Option<T>` for legitimate absence and `Result<T, E>` for failure. Use `Result<Option<T>, E>` when both matter. Do not turn provider/storage failures into absence with `.ok()` or `unwrap_or_default()`.
- Validate related fields together: memory bounds, modded loader/version selection, quick-play target, and version-change target. Prefer a type that prevents contradictory selections where practical.
- Keep `serde_json::Value` for genuinely variable data. Deserialize stable provider fields and string-or-list manifest arguments into explicit types. Share wire and domain types when their semantics match; separate them when validation or compatibility requires conversion.

**5. Preserve meaningful errors**

- Use small `thiserror` enums for errors callers need to distinguish. Keep I/O/HTTP/etc. source errors with `#[source]` or `#[from]`, adding operation context where needed.
- Use `anyhow` only at reporting boundaries such as startup or tooling. Do not make it the return type of every feature service, and do not replace string errors with a universal `Message(String)` variant.
- Map internal errors to a stable serializable API error at the IPC boundary. Do not serialize native errors or erase their categories through a blanket `to_string()` serializer. Local diagnostics retain causes; user-facing errors exclude tokens, passwords, raw auth responses, and secret command arguments.
- Do not add `unwrap`, `expect`, `panic!`, or unchecked indexing on paths driven by user input, files, providers, or concurrency. A documented internal invariant or isolated startup failure may justify a panic. Test assertions may unwrap.
- Propagate required-operation failures. Best-effort cleanup may be nonfatal only when its policy is explicit; provide useful diagnostics without reporting already committed work as failed.
- Preserve primary and rollback errors when both occur. Never claim success or successful recovery when required work failed. Retain recovery material until recovery is confirmed.
- Transitional string errors may remain only at an explicitly tracked legacy boundary. New internal code uses typed errors; remove the conversion when that boundary migrates.

**6. Treat state changes and async work deliberately**

- Distinguish missing, corrupt, and unreadable persistence. Missing data may use documented defaults; corruption must not silently become an empty store that is then overwritten.
- Serialize repository read-modify-write operations to prevent lost updates. Stage and replace files with platform-tested semantics. Atomic replacement of one file does not make a multi-file operation transactional.
- Account mutations use `AppState.accounts` transactions. Provider results patch freshly loaded profiles; do not reintroduce a save API for an earlier account-list snapshot. Keep network and credential-vault effects outside the repository lock.
- Account credential writes, refreshes, skin changes and removal use `AppState.account_operations` ownership plus the managed credential adapter. Token helpers reuse the caller's guard rather than reacquiring it. Preserve newly rotated credentials on profile failure and report partial commits explicitly; do not claim rollback or restart recovery without implementing and testing it.
- Native account profile reads and mutations pass through the application-owned recovery gate. Publish credential/profile changes as typed recovery intents; never replace this with an opaque closure. Keep the ordered pending queue and secrets in the vault, and only the discovery marker on disk. Acknowledge completion before deleting a chunked journal. Recovery locks contain synchronous local effects only: no providers or awaits, and no vault calls inside the repository file lock. Preserve invalid/unavailable recovery data and report the blocked operation; do not discard it as an empty store.
- Account sign-in/token orchestration uses the Tauri-independent session service and application-owned SessionProvider. Keep provider transport/status/parse failures distinct from token expiry; an unavailable validation provider must not trigger refresh. Test protocol adapters against loopback endpoints and session state changes with controlled fake futures. Use typed Microsoft stage errors and injected client/endpoints/clock; do not restore the string-protocol adapter or reflect raw provider error descriptions into RPC errors.
- Microsoft completion claims only the current application's issued device code once. Use its original monotonic deadline and trusted polling fields; do not renew expiry from frontend input or capture generation when finish arrives. Failed/abandoned completion consumes that code; only a retained validated grant can be explicitly resumed. Serialize cancellation/new starts with synchronous publication; lock order is account ownership, sign-in publication, recovery, repository. Never hold the sign-in publication guard across an await or acquire it inside recovery/repository effects.
- Existing-account Microsoft refresh returns a validated grant before Xbox/Minecraft exchange. Checkpoint its refresh token synchronously through recovery before another await, preserving old Minecraft credentials but marking them expired. Stop on checkpoint failure; retries use the recovered refresh token. Never store the Microsoft OAuth access token in the Minecraft access-token field or reintroduce a combined refresh/exchange call that bypasses this boundary.
- Device-code redemption also returns a validated grant before exchange. Checkpoint its refresh token in PendingLoginGrants under the sign-in publication guard; never put unidentified accounts in the profile recovery queue. New starts/cancellation invalidate the generation and acknowledge disposal under the same guard. Retained grants never auto-activate after restart. Explicit resume requires a matching version-2 ticket within 24 hours of its original checkpoint and an exclusive generation-bound lease, released on failure or dropped futures. Under sign-in ownership, recover account intents before inspecting/claiming/discarding a grant; checkpoint every fresh refresh token before exchange without extending retention. Keep legacy/expired records explicitly disposable; preserve malformed/unknown records and report cleanup failures, including after an account was already published.
- Transfer an identified Microsoft sign-in using its scope/ticket receipt in the durable account journal. All pending-grant effects use AccountRecovery's separate synchronous pending-login gate so profile recovery cannot overwrite a concurrent new ticket; never call recovery or acquire sign-in ownership from inside that gate. Recovery acknowledges only the intent's ticket before publishing canonical credentials or the profile. Preserve version-1 journals and ambiguous legacy grant records, and never infer that a legacy grant is resumable. A handoff failure blocks account effects while retaining the journal; failed deletion after acknowledgement leaves an inert grant.
- Stage and verify downloads/imports before replacing a working installation. Validate supplied checksums as well as sizes. Apply one explicit archive/path containment policy, including Windows path forms and symlink/junction behavior.
- For multi-step changes, define commit, rollback, cancellation, and restart recovery. Include metadata/tracking in the consistency model. Persist sufficient recovery information before destructive operations.
- Use `async` for asynchronous work. Put blocking scans, hashing, archive/image/NBT processing, and bulk copying behind a deliberate blocking boundary; handle task join errors separately from domain errors.
- Never hold a synchronous mutex guard across `.await`. Keep async lock scopes narrow and avoid holding shared registry locks during network or process I/O. A lock intentionally spanning an async protocol must have documented ownership and ordering.
- Give spawned tasks an owner and a shutdown/cancellation policy. Cleanup must not rely only on code after an awaited operation that might be cancelled. Use guards where necessary for reservations and pending-query counters.
- Claim start operations atomically; preserve generation/PID identity checks before stale tasks mutate state or terminate a process. Test start/stop/cancel races with controlled fakes.

**7. Preserve contracts while splitting TauRPC**

- Give each feature procedures trait a unique path and merge it once. Do not use two traits with the same path to split declarations; duplicate paths can replace registrations.
- Track command and event moves. Update Rust handlers, backend event emitters, frontend service calls, direct component calls, subscriptions, and generated bindings together.
- Keep method/argument names and serialized shapes stable during initial moves. Change names or DTOs in a deliberate migration with compatibility tests, not as incidental cleanup.
- Regenerate `../src/bindings.ts` using a reproducible export path; never hand-edit it. Export errors fail verification. Once implemented, export without launching the UI or running application setup side effects.
- Enabling TauRPC result mode is a coordinated contract change. Update React Query, success/error branches, and notifications; do not let an error envelope enter a success path. Keep unexpected transport rejections typed as `unknown` and validate before narrowing.
- Keep temporary compatibility adapters small and delegating to the same implementation. Record their removal milestone. Do not duplicate business logic or emit duplicate events.
- Preserve stored field names/defaults, account ID prefixes, keyring service names, and existing instance/server/backup formats. Introduce explicit migration tests when a format must change. Snapshot/release identifiers must not be forced into semver.
- Preserve Windows, Linux, macOS, and Android behavior. Localize platform branches in adapters. Do not change generated Android files, bundled JARs, icons, or native binaries merely to reorganize Rust.
- Add only dependencies actually used by the current change. Preserve unrelated dependency pins and the TauRPC/Specta pairing; update Cargo.lock when a required dependency change makes it necessary.

**8. Test behavior and failure paths**

- Add unit tests for extracted decisions, validation, and state transitions. Before substantial moves, use existing tests or add characterization tests for the behavior being preserved. Test fixes with a regression that fails before the fix.
- Assert meaningful outputs, state, events, and error variants. Do not assert implementation call sequences or exact English error text unless those are the contract. Avoid tests that only duplicate implementation logic, getters, or trivial forwarding.
- Default tests use no public network, real credentials/vault, installed Minecraft/Java, or actual game/server process. Use fixtures, a fake secret/process provider, local HTTP servers, and unique `tempfile` directories.
- Use controlled time for polling, expiry, throttling, and stop timeouts. Avoid arbitrary sleeps, shared temp paths, fixed ports, global environment mutation, and dependence on test execution order. Add Tokio `test-util` when paused time is needed.
- Keep pure tests local to their feature. Use local integration tests for adapters, transactions, and the IPC contract. Keep real-vault/provider/platform smoke tests explicitly opt-in.
- Cover failure injection for destructive workflows: download, copy, write, rename, commit, rollback, and cancellation. Verify the previous data or recovery material survives.
- Follow the plan's test matrix and coverage ratchet. Coverage targets complement scenario coverage; do not inflate percentages with trivial tests or exclude difficult feature code. Do not claim measured coverage without a report.

**9. Verify and leave a useful handoff**

For Rust implementation changes, run focused tests first, then the applicable checks below once the change is stable. Run Cargo commands from `src-tauri` and frontend commands from the repository root.

```text
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
pnpm exec tsc --noEmit -p .
pnpm test:ipc
```

- The five live API tests are explicitly ignored smoke tests. Default tests run without skip filters; opt in to a specific live test by name with `--ignored --exact`. Cargo's `--offline` restricts dependency access; it does not disable network calls made by tests.
- Binding freshness and the procedure inventory are checked by `ipc_contract::generated_bindings_match_the_committed_contract` in the library suite. Regenerate without launching the app:

```text
cargo test --lib --locked ipc_contract::export_bindings -- --ignored --exact
```

- Register APIs through the single list in `src/ipc/mod.rs`; runtime routing and metadata-only export both consume it. During each namespace migration, update inventory `currentPath`, backend event triggers, frontend callers, and generated bindings together. Keep unmigrated commands in `ipc/legacy.rs` only until their feature migrates.
- IPC changes run both the Rust mock-runtime boundary tests and `pnpm test:ipc`, which exercises generated bindings through the installed TauRPC runtime. Use the Node version in `.nvmrc` for the latter; it reads generated TypeScript directly.

- Run TypeScript checks and binding freshness verification for IPC/DTO/consumer changes. Run applicable frontend lint checks for changed frontend files. Add platform build checks and coverage measurement when the task changes those areas or completes the corresponding milestone.
- Record pre-existing formatting/lint/test failures separately from regressions. Fix regressions; do not add broad lint allowances, change assertions, or disable tests to obtain a green run. Keep unrelated formatting cleanup out of the change.
- If a tool or target is unavailable, report the exact unverified check and its reason. Do not present unrun cross-platform tests, coverage, or export verification as passed. Documentation-only changes need document/link/diff review, not an application build.
- Inspect the final diff and working tree. Report scope, observable behavior changes, compatibility, commands/results, remaining migration adapters, and the next concrete step. Update the relevant plan checkpoint without rewriting its historical baseline.
- A slice is complete when its acceptance criteria and applicable checks are satisfied. Smaller files alone do not establish completion.
