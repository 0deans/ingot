# Handoff: safe version change, steps 2 and 3

You are continuing a feature in **Ingot**, a Tauri 2 + React + Rust Minecraft launcher
and server host (Windows desktop + Android). Step 1 is done and tested. Your job is
steps 2 and 3 below. Read this whole file before writing code.

---

## 0. Rules of this project (follow strictly)

**Behaviour the owner insists on**
- Safety first: a version change must never leave an instance/server half-changed. Every
  change is all or nothing and can be undone exactly.
- Nothing is deleted when it can be turned off: disabled files are renamed to
  `<name>.disabled`; replaced files are moved into the backup.
- No leftover code: no debug code, no "just in case" migrations, no compatibility shims
  for old data. There are no real users yet.
- Never make the game/server save the world or change its config to make a feature work.
- Match the surrounding code (naming, comment density, idioms). Keep changes minimal.
- Write user-facing text in plain, short English. No jargon.

**Tech conventions**
- IPC uses **taurpc + specta**. The RPC trait and impls are in `src-tauri/src/ipc.rs`
  (declaration block at the top, `impl` further down; copy the style of the
  `*_instance_version_*` functions).
- `src/bindings.ts` is **generated**. Never hand-edit it. To regenerate: `cargo build`
  (in `src-tauri`), then run `target\debug\ingot.exe` for ~5 s until `src/bindings.ts`
  changes, then kill that process. Only kill processes whose path is under
  `target\debug` — **never** the user's installed `C:\Program Files\ingot\ingot.exe`.
- specta: every `f64`/`u64` number field in a type sent to the frontend needs
  `#[specta(type = i32)]`, otherwise it becomes `number | null` in TypeScript.
- Frontend: React + Tailwind v4. The theme has `--radius: 0` (square corners) on the
  launcher side. Import `cn` from `"cn"` in `src/components/instances/*`.
- Checks that must pass before you finish:
  - `pnpm exec tsc --noEmit -p .`
  - `pnpm exec biome ci` (auto-fix with `pnpm exec biome check --write <files>`)
  - `cargo test --lib` (in `src-tauri`)
  - `cargo check` for Android (the Rust code is shared):
    ```
    NDK="$LOCALAPPDATA/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/windows-x86_64/bin"
    CC_aarch64_linux_android="$NDK/aarch64-linux-android24-clang.cmd" AR_aarch64_linux_android="$NDK/llvm-ar.exe" cargo check --target aarch64-linux-android
    ```
- Testing the real app without touching the user's data: build with
  `TAURI_CONFIG='{"identifier":"ingot-dev"}' cargo build`, create test data in
  `%APPDATA%\ingot-dev`, run Vite with `pnpm exec vite --port 1420 --strictPort`, start
  `target\debug\ingot.exe` with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223`
  and drive it over CDP. Afterwards: kill only `target\debug` processes and the port-1420
  process, delete `%APPDATA%\ingot-dev` and `%LOCALAPPDATA%\ingot-dev`, then
  `cargo clean -p ingot && cargo build` and confirm `grep -c ingot-dev target/debug/ingot.exe`
  prints `0`.
- Network tests go in `#[ignore]` tests or temporary tests you delete afterwards.
  Permanent tests must not need the network.

---

## 1. What step 1 already built (instances, Modrinth)

**Engine:** `src-tauri/src/minecraft/version_change.rs`
- `check(client, instance_dir, &InstanceConfig, to_game_version, to_loader_version, downgrade) -> VersionPlan`
  - Scans enabled files in `mods/` (`.jar`), `resourcepacks/` and `shaderpacks/` (`.zip`).
  - Hashes them with SHA-1 and looks them up on Modrinth:
    - `POST /v2/version_files` for what each file is now.
    - `POST /v2/version_files/update` with `loaders` + `game_versions` for its target version.
    - If the target is an alpha/beta, `best_version()` looks for a stable release.
  - Gives each item an `ItemStatus`: `works | update | missing | unknown | newDependency | conflict`.
  - Gives each item a recommended `ItemAction` (`keep | update | disable | add | skip`)
    plus the list of allowed `actions`.
  - Duplicates (same project twice) → `conflict`.
  - `resolve_dependencies()` repeats until nothing changes:
    - Adds missing *required* dependencies as `newDependency`.
    - Flags *incompatible* pairs.
    - Flags mods whose required dependency isn't available.
  - Notes reference projects by id; ids are replaced with titles at the end.
  - Unknown jars get their name from `crate::server::plugins::read_jar_metadata`, which reads
    `fabric.mod.json`, `quilt.mod.json`, `(neoforge.)mods.toml` and `plugin.yml`.
- `apply(client, instance_dir, &InstanceConfig, &VersionPlan, backup_worlds, progress) -> InstanceConfig`
  1. `stage()`:
     - Downloads every `update`/`add` target into `.ingot/version-change/staging/<folder>/`, 6 at a time.
     - Only `cdn.modrinth.com` URLs are accepted, and each file is verified by SHA-1.
     - If anything fails, nothing has changed.
  2. `swap()`:
     - Copies `config/`, `options.txt` and, optionally, worlds (`saves/<world>`) into
       `.ingot/version-change/backup/`.
     - Moves replaced files into `backup/files/...`, renames disabled files to `.disabled`,
       and moves staged files into place.
     - Every step is recorded in a `Journal`.
  3. On any error, `restore(journal)` undoes what was done. On success, `journal.json` is written.
- `last_backup()`, `undo()` (refuses if the instance's version is no longer the one the
  backup changed it to), `discard_backup()`. Only the **last** change is kept.
- `safe_file_name()` rejects paths and wrong extensions. Tests:
  - `swaps_and_undoes_exactly` (real files, no network)
  - `rejects_unsafe_file_names`

**IPC** (`src-tauri/src/ipc.rs`):
- `check_instance_version_change(instance_id, game_version, loader_version)`
  - Detects a downgrade by comparing Mojang manifest `release_time`.
- `apply_instance_version_change(plan, backup_worlds)`
  - Refuses while the game runs, then saves the new `InstanceConfig`.
- `get_instance_version_backup(instance_id)`, `undo_instance_version_change(instance_id)`,
  `discard_instance_version_backup(instance_id)`.

**UI**
- `src/components/instances/change-version-dialog.tsx`:
  - Picker, then review list grouped by status with per-row action switches.
  - World backup checkbox.
  - Red downgrade warning that disables apply until "I understand".
  - Applying spinner, then "Now on X".
- `src/components/instances/instance-settings-dialog.tsx`:
  - "Minecraft version" section with "Change version", plus "Undo, back to X" / "Delete backup" when a backup exists.
  - The change dialog is mounted only while open.
  - "Save Changes" re-reads the instance first, so it can't overwrite a version change.

Related code you will need:
- `src-tauri/src/minecraft/instance.rs`: `InstanceConfig { game_version, loader: ModLoaderType, loader_version, ... }`, `update_instance`
- `src-tauri/src/minecraft/loader.rs`: `fetch_loader_versions(client, loader, game_version)`
- `src-tauri/src/minecraft/launcher.rs`: game launch; the task after `child.wait()` (around line 840) handles game exit
- `src-tauri/src/minecraft/content.rs`: CurseForge search/install. Has `CURSEFORGE_API_KEY` (header `x-api-key`).
- Servers:
  - `src-tauri/src/server/config.rs`: `ServerConfig`, `ServerCoreType` (Paper, Purpur, Fabric, Vanilla, Folia, Pumpkin, NeoForge, Forge, Quilt).
  - `src-tauri/src/server/plugins.rs`:
    - `platform(core)` returns folder `plugins`/`mods` and the Modrinth loaders; the fields are private, so make them `pub(crate)` if needed.
    - Also: Hangar support, and tracking of installed items in `.ingot/plugins.json` (`TrackedPlugin`).
    - `install()`, `versions()`, `restore_missing()`.
  - `src-tauri/src/server/transfer.rs`:
    - `change_version()` (the OLD, unsafe server version change).
    - `duplicate()`, export/import.
  - `src-tauri/src/server/installer.rs`: NeoForge/Forge/Quilt install on first start. `uninstall(dir, core)` forces a reinstall.
  - `src-tauri/src/server/downloader.rs`: `fetch_core_versions`, `ensure_server_jar` (downloads `server.jar` if missing).
  - `src-tauri/src/server/process.rs`: server start. The line `Done (` … `For help` marks "started".
  - `src-tauri/src/server/map.rs`: `level-name` world folder lookup.
  - Old server UI: `ChangeVersionDialog` in `src/components/servers/panels/transfer-card.tsx`. It optionally makes a backup copy with `duplicate_server`, then calls `change_server_version`.

---

## 2. Task A: servers use the same engine, plus CurseForge fingerprints

### A1. Make the engine target-agnostic
Extract what is instance-specific out of `version_change.rs` into a small description of
the target, for example:

```rust
pub struct ChangeTarget {
    pub root: PathBuf,                    // instance dir or server dir
    pub folders: Vec<(ContentKind, &'static str, Vec<&'static str>)>, // kind, folder, loader names
    pub game_version: String,
    pub worlds: Vec<String>,              // folders to back up when asked
    pub config_paths: Vec<String>,        // files/folders copied into the backup
}
```

- **Instance:** `mods` + `resourcepacks` + `shaderpacks`; worlds = `saves/*`; configs = `config/`, `options.txt`.
- **Server:**
  - One folder, `plugins` or `mods` (from `plugins::platform(core)`), with its Modrinth loaders.
  - Worlds: the `level-name` world plus `<name>_nether` and `<name>_the_end` if present.
  - Configs:
    - `config/`
    - the top-level `*.yml`, `*.yaml`, `*.toml`, `*.properties` and `*.json` files (Paper rewrites `bukkit.yml`, `paper-global.yml` and others on upgrade)
    - each `plugins/<Name>/` config folder (not the jars)
    - `.ingot/plugins.json`
- Keep instance behaviour identical. The existing tests must still pass. Move the backup work
  dir so both use `<root>/.ingot/version-change/`.

### A2. Server-specific parts
- **Plugins from Hangar** (Paper/Purpur only, tracked in `.ingot/plugins.json` with `source: hangar`):
  - Hangar has no hash lookup.
  - For tracked Hangar items, use `plugins::versions(core, target, Hangar, project_id, true)` and `plugins::pick_version`.
  - Treat untracked, unknown jars as `unknown`.
- **Server core files:** the backup must let undo restore the old server exactly.
  - Move `server.jar` into the backup (the new one downloads on next start via `ensure_server_jar`).
  - For NeoForge/Forge/Quilt, also move what `installer::uninstall` would delete (`libraries/net/neoforged/neoforge`, `libraries/net/minecraftforge/forge`, or `quilt-server-launch.jar` + `server.jar`) into the backup instead of deleting it.
  - Update `ServerConfig.game_version` and `build_number` (set `build_number` to `None` so the newest build for the new version is used).
- After the swap, update `.ingot/plugins.json` entries for updated/added items (file name, version id, version number). The backup copy restores it on undo.
- Refuse unless the server is **stopped** (see how `change_server_version` checks `get_server_process_manager()`).
- New IPC, mirroring the instance ones:
  - `check_server_version_change(server_id, game_version)`
  - `apply_server_version_change(plan, backup_worlds)`
  - `get_server_version_backup`, `undo_server_version_change`, `discard_server_version_backup`
- Remove the old `change_server_version` RPC and `transfer::change_version` once nothing uses them (no leftovers). Keep `duplicate_server` ("Make a copy"); it's a separate feature.
- **UI:**
  - Make the review part of `change-version-dialog.tsx` reusable, e.g. a `VersionPlanReview` component taking the plan plus choices.
  - Use it in the server `ChangeVersionDialog` in `transfer-card.tsx`. Server panels use rounded styles (`rounded-xl`/`rounded-2xl`) and `components/servers/shared/primitives`.
  - Keep the server picker: versions come from `get_available_server_core_versions(core)`.
  - Show "Undo, back to X" on the Backup & version card when a backup exists.

### A3. CurseForge fingerprints (both instances and servers)
Files not found on Modrinth get a second chance on CurseForge:
- **Fingerprint** = MurmurHash2 (32-bit, seed `1`) of the file bytes **with every byte 9, 10,
  13 and 32 removed**. Write it as a small pure function.
  - Algorithm, on the filtered bytes of length `len`:
    - `m = 0x5bd1e995`, `r = 24`, `h = 1 ^ len` (wrapping u32 math).
    - For each full 4-byte **little-endian** chunk `k`: `k *= m; k ^= k >> r; k *= m; h *= m; h ^= k`.
    - Tail of 1–3 bytes: XOR them in (`h ^= b[2] << 16`, `h ^= b[1] << 8`, `h ^= b[0]`), then `h *= m`.
    - Final: `h ^= h >> 13; h *= m; h ^= h >> 15`.
  - Verify it against the real API once in an `#[ignore]` test. Download any mod file from CurseForge, fingerprint it, and check `/v1/fingerprints/432` returns it in `exactMatches`. Then hard-code that file-independent case plus a few small byte strings as unit tests.
- `POST https://api.curseforge.com/v1/fingerprints/432` with body `{"fingerprints":[u32,...]}` → `data.exactMatches[]`
  - Each match gives `id` (mod id) and `file` (current file: `displayName`, `gameVersions`, `dependencies`, `releaseType`).
- Target version: `GET /v1/mods/{modId}/files?gameVersion=<mc>&modLoaderType=<n>` → pick the newest `releaseType == 1` (release), else beta (2), else alpha (3).
  - `modLoaderType`: Forge 1, Fabric 4, Quilt 5, NeoForge 6.
  - For Bukkit plugins, CurseForge `gameId=432` classId differs. Only do CurseForge for mods, resource packs and shaders; skip it for server plugins.
- Titles/icons: `POST /v1/mods` with `{"modIds":[...]}` → `name`, `logo.thumbnailUrl`.
- Dependencies: `file.dependencies[] { modId, relationType }`. Required = 3, incompatible = 5, optional = 2, embedded = 1. Feed these into the same dependency resolution.
- **Downloads:**
  - `file.downloadUrl` can be **null** (the author disabled third-party downloads). Then the item can't be updated automatically: mark it `missing` with the note "Download it from CurseForge yourself", offer only Keep/Turn off, and add a link to the mod page.
  - Hashes: `file.hashes[] { value, algo }` with algo 1 = SHA-1. Verify SHA-1 like Modrinth.
  - Extend the host allow-list in `stage()` to `edge.forgecdn.net` and `mediafilez.forgecdn.net`.
- Add a `source: "modrinth" | "curseforge"` field to `PlanItem` so the UI can show where an item comes from and link to it.
- Always send the `x-api-key` header, reusing `CURSEFORGE_API_KEY` from `content.rs` (move it to a shared place, not a copy).

### A acceptance
- Paper server 1.21.4 with 3 Modrinth plugins, 1 Hangar plugin and 1 unknown jar, checked against a newer version, shows the right statuses.
- Apply, start the server, and it reaches `Done`. Undo gives back the exact jars, configs, `server.jar` and world (compare file hashes).
- A Fabric instance with one CurseForge-only mod (for example from a CurseForge modpack) shows it as `update`/`works` instead of `unknown`.
- A CurseForge file with `downloadUrl: null` is never downloaded and the UI says why.

---

## 3. Task B: loader switching, and a crash check after the first start

### B1. Loader switching (instances first, then servers)
- Add an optional target loader to `check`. When it differs from the current one:
  - For every identified mod, find the same **project** for the new loader (`best_version(project, target_game, new_loaders)` on Modrinth, or `modLoaderType` on CurseForge). Found → `update`; not found → `missing` (Turn off).
  - Remove loader-specific libraries that have no meaning on the new loader instead of flagging them missing: Fabric API / Quilted Fabric API when leaving Fabric/Quilt, and the reverse. Show them as a normal `disable` item with the note "Only needed on Fabric".
  - Quilt runs Fabric mods: Fabric → Quilt keeps Fabric mods as `works`.
  - Unknown jars are always `unknown`/Turn off when the loader changes.
  - Vanilla → modded: nothing to check except resource packs/shaders.
  - Modded → vanilla: all mods are turned off.
- The picker gets a loader choice (reuse the `LOADERS` list and `LoaderIcon` from `new-instance-dialog.tsx`), then loads loader versions for the chosen loader.
- Apply must also save `InstanceConfig.loader`. The journal must remember the old loader, and undo restores it.
- **Servers:** a core change is only offered between plugin-compatible cores (Paper ↔ Purpur ↔ Folia, where Folia accepts only Folia plugins) and between mod loaders (Fabric/Quilt/NeoForge/Forge).
  - Changing between the plugin world and the mod world means a new server; don't offer it.
  - Moving `server.jar`/installer files into the backup (A2) covers the core files.

### B2. Crash check after the first start
- In the journal, add `first_start_checked: bool` (false after apply).
- **Instances:**
  - In `launcher.rs`, in the task that waits for the game to exit, look at the version-change journal of that instance when it exists and `first_start_checked` is false.
  - The game counts as crashed if the exit code is non-zero, **or** a new file appeared in `crash-reports/` since launch, **or** the play time was under ~20 s with a non-zero code.
  - On a clean exit, set `first_start_checked = true` and do nothing else.
- **Servers:**
  - In `process.rs`, the same check when the process exits while the status is still Starting (no `Done (` line yet) after a version change.
  - Set `first_start_checked` when `Done (` is seen.
- On a crash:
  - Read the newest crash report, or the last ~200 lines of `logs/latest.log` if there is none.
  - Find suspected mods:
    - Fabric/Quilt crash reports have a `Suspected Mod(s):` / `Suspected Mods:` section with mod ids.
    - Forge/NeoForge have `-- Mod loading issue for: <modid> --` and `Mod File: <file>` lines.
    - Map mod ids to plan items by reading each jar's metadata (`read_jar_metadata` gives the name; extend it to return the mod id too).
  - Emit a taurpc event, e.g. `on_version_change_crash { target_id, kind: instance|server, suspects: [{ title, file_name }], report_excerpt }`. See the other `#[taurpc(event)]` declarations in `ipc.rs`.
- UI: a dialog when the event arrives:
  - "The game crashed on its first start on 26.3."
  - List the suspected mods, each with "Turn off".
  - Buttons: "Undo the version change" and "Keep trying".
  - Turning a mod off is a normal rename to `.disabled`, and it is also recorded in the journal so undo stays exact.
- Never auto-undo without the user's click.

### B acceptance
- Fabric instance: switch to Quilt keeps mods; switch to NeoForge offers NeoForge builds of the same projects and turns off Fabric API. Undo restores the loader and files exactly.
- Put a jar known to crash on the target version into a test instance, apply, and launch: the crash dialog appears and names that mod. "Undo" restores everything.
- A clean first start sets `first_start_checked` and never shows the dialog again.

---

## 4. Order of work and finishing
1. A1 refactor (instance tests still pass).
2. A2 servers + UI.
3. A3 CurseForge.
4. B1 loader switching.
5. B2 crash check.

After each step, run all checks from section 0 and test in the isolated `ingot-dev` app.
At the end, give the owner a short, plain summary of:
- what works
- what was tested and how
- what wasn't tested (for example on the phone)
