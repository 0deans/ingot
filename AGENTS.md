**Repository guidance**

- Read `../AGENTS.md`, when present, for shared coding preferences. Repository-specific instructions take precedence over those shared defaults.
- For Rust backend work, read `src-tauri/AGENTS.md` before making changes. Also apply its compatibility and verification rules to frontend or CI edits made as part of that backend work.
- For the Rust refactor, read the relevant phase of `RUST_REFACTOR_PLAN.md`. That file describes scope and sequencing; `src-tauri/AGENTS.md` defines coding and review expectations.
- Follow the current user request. A roadmap or request to improve these instructions does not itself authorize implementing the refactor. Once implementation is requested, carry out its authorized scope without asking again for routine implementation choices.
- Inspect the working tree before editing. Preserve unrelated staged, unstaged, and untracked changes; do not reset or reformat another contributor's work.
- Do not stage, unstage, or commit any changes, including your own, unless the user explicitly requests that Git action. A request to edit code or fix an issue does not authorize staging, unstaging, or committing.
- Do not add `Co-authored-by` trailers, AI/agent credits, or generated-by attribution to suggested commit messages or actual commits unless the user explicitly requests that attribution. Asking for a commit message authorizes drafting text only, not staging or committing.
- Keep generated bindings generated. Changes to the Rust IPC contract must include regeneration and affected TypeScript callers; do not hand-edit `src/bindings.ts`.
- Keep these instructions concise and update stale guidance when the implementation changes. Do not duplicate the whole refactor plan here.
