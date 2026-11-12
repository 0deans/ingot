# Walkthrough: Minecraft Launch Engine, Multi-Loader Support & Concurrent Process Tracking

We have implemented a high-performance, multi-loader Minecraft launch engine for Ingot supporting **Vanilla**, **Fabric**, **Quilt**, **NeoForge**, and **Forge**, featuring **chunked multi-threaded file downloads**, **automatic Java runtime resolution**, and **true concurrent multi-instance execution**.

---

## What Was Built

### 1. High-Speed Multi-Threaded Chunk Downloader (`minecraft/downloader.rs`)
- **Large Files (`client.jar`, Java runtime zips > 4MB)**:
  - Probes server for `Accept-Ranges: bytes`.
  - Splits file into 4MB ranges and downloads across parallel Tokio worker tasks.
  - Pre-allocates destination file and writes chunks directly at byte offsets.
- **Small Files (assets & libraries, typically 500–2,000 files)**:
  - Concurrent connection pool bounded by `tokio::sync::Semaphore(24)`.
  - SHA-1 checksum verification on every downloaded file.
  - Global shared cache (`<app_data>/libraries/` and `<app_data>/assets/objects/`) to prevent redundant re-downloads across instances.

### 2. Automatic Java Runtime Resolution & Auto-Download (`minecraft/java.rs`)
- **Metadata Version Matching**:
  - Automatically matches the required major version (`javaVersion.majorVersion`):
    - Minecraft < 1.17 $\rightarrow$ Java 8
    - Minecraft 1.17 $\rightarrow$ Java 16
    - Minecraft 1.18 – 1.20.4 $\rightarrow$ Java 17
    - Minecraft 1.20.5+ $\rightarrow$ Java 21
- **Automatic Fallback Download**:
  - Checks if a compatible Java runtime exists locally (or in `<app_data>/runtimes/java-{major}/`).
  - If missing, automatically downloads the certified Eclipse Adoptium (Temurin) JDK package via `https://api.adoptium.net/v3/binary/latest/{major}/ga/windows/x64/jdk/hotspot/normal/eclipse`.
  - Uses chunked downloading, extracts the archive directly using the Rust `zip` engine, and caches it globally so Java is only downloaded once per major version.

### 3. Universal Mod Loader Resolution (`minecraft/loader.rs` & `version.rs`)
- **Vanilla**: Mojang Version Manifest V2 + version package + asset index + platform OS/arch rule evaluator.
- **Fabric**: Official Meta API (`meta.fabricmc.net`) resolving profile JSON, loader libraries, and `KnotClient` entrypoint.
- **Quilt**: Official Meta API (`meta.quiltmc.org`) resolving profile JSON and libraries.
- **NeoForge & Forge**: Prism Meta API (`meta.prismlauncher.org/v1/`) resolving modern pre-packaged libraries and entrypoints without requiring external Java installers.

### 4. True Multi-Instance Concurrency & Process Manager (`minecraft/launcher.rs`)
- Global asynchronous process tracker (`Arc<Mutex<HashMap<String, ActiveChild>>>`).
- Users can launch and run **multiple instances simultaneously** (e.g. Vanilla and Fabric concurrently).
- Each running instance has its own isolated directory (`<app_data>/instances/<id>/`), dedicated PID, live duration clock, and individual Stop/Kill button.
- Process exit monitoring in a background Tokio task with automatic status broadcasting and playtime tracking.

### 5. Frontend & UI Controls
- **`NewInstanceDialog`**:
  - Dynamic Minecraft version list (with snapshot toggle).
  - Mod Loader selector tabs (Vanilla, Fabric, Quilt, NeoForge, Forge).
  - Dynamically fetches available loader versions upon selecting a game version.
- **`InstanceHero`**:
  - **Ready**: Green badge + Play button.
  - **Downloading / Preparing**: Progress bar showing download phase, progress details, and percentage.
  - **Running**: Pulsing green indicator, PID, active elapsed playtime clock, and a "Stop Game" button.
- **Multi-Instance Banner & Library Grid (`index.tsx`)**:
  - An active banner displays all running processes with quick Stop buttons.
  - An instance card library allows easy switching between instances and launching instances in parallel.

---

## Verification Results

### Automated Build Checks
1. **Rust Backend (`src-tauri`)**:
   - `cargo check`: Passed with exit code 0 and **0 warnings**.
2. **Frontend (`src`)**:
   - `pnpm check`: Passed with exit code 0 across all 50 files.
   - `pnpm build`: TypeScript typecheck and Vite build succeeded with exit code 0 (`dist/index.html` generated).

---

## Launch Sequence Stall Diagnosis & Fix

When testing instance launch, the UI was previously getting stuck at `"Initializing launch sequence..."`. Two root causes were identified and fixed:

1. **TauRPC Event Serialization Mismatch**:
   - The frontend TauRPC client expects an event payload structure of `{ proc_name, input_type: ev }`.
   - The backend was calling raw `taurpc::EventTrigger::call(...)` which emitted `{ event_name, event: ev }`.
   - Because `input_type` was undefined, the frontend event handler threw an exception on receipt of progress events, leaving the progress state stuck at step 1.
   - **Fix**: Replaced raw trigger calls with macro-generated `TauRpcAppApiEventTrigger::new(app.clone()).on_launch_progress(ev)` and `.on_instance_status_changed(ev)`.

2. **Chunk Downloader Redirection Overhead**:
   - The Adoptium API endpoint (`api.adoptium.net`) issues 307/302 redirects to GitHub/CDN mirrors.
   - When downloading Java runtimes via chunk ranges, each chunk task was re-querying the original redirect URL, causing dozens of concurrent redirected requests to stall.
   - **Fix**: The downloader now follows the `HEAD` probe's final redirected URL and performs range requests directly on the final content mirror.

3. **Frontend UI Null-Safety & Error Handling**:
   - Wrapped `rpc.launch_instance` with `try...catch` cleanup so progress state resets if an IPC call fails.
   - Handled nullable `percentage` in `InstanceHero` for indeterminate progress steps.

---

## Launch Argument Fix, Java 25 Detection & Instance Hero Redesign

1. **Fixed Minecraft Launch Crash (`IllegalArgumentException: Only one quick play option can be specified`)**:
   - Filtered out unreplaced game arguments (such as `--quickPlayPath`, `--quickPlaySingleplayer`, `--quickPlayMultiplayer`, `--quickPlayRealms`, and `--demo` when not configured).
   - Injected critical modern JVM access flags (`--sun-misc-unsafe-memory-access=allow`, `--enable-native-access=ALL-UNNAMED`) and configured native libraries directory.
   - Directed Minecraft stdout/stderr directly through `Stdio::inherit()` to avoid stream buffering overhead.

2. **Accurate Java 25 Detection**:
   - Resolved Java major version detection for snapshot `26.2` and modern releases to correctly show **Java 25 (Adoptium)** instead of defaulting to Java 8.

3. **Instance Hero Redesign**:
   - Eliminated all generic marketing placeholder text.
   - Built a sleek, gamer-focused dashboard featuring a 4-card specs grid:
     - **Minecraft Version**: Game version tag
     - **Java Runtime**: Accurate Java version (e.g. `Java 25 (Adoptium)`)
     - **Memory**: Min / Max allocated RAM
     - **Total Playtime & Last Played**: Live calculated playtime duration and relative last-played timestamp
   - Added a quick **Folder** button in the header to open the instance directory in File Explorer (`open_instance_folder` IPC).

