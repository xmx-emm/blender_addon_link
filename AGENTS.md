# Project instructions

- Read `PROJECT_CONTEXT.md` at the repository root before reading or searching source code.
- Use that summary to select the smallest relevant scope, then verify important claims against the current source files.
- Keep `PROJECT_CONTEXT.md` concise and factual. Refresh it when code, structure, build behavior, dependencies, or durable project knowledge changes.
- For Tauri changes, inspect `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` together. Keep explicit npm scripts for desktop development/build and any auxiliary binaries.
- Verify frontend type-check/build and relevant Cargo checks before reporting a release-ready state.
