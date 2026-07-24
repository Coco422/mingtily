# Mingtily Desktop App

The supported Mingtily application is a Tauri 2 desktop app with a Next.js/React frontend and a Rust native core.

## Prerequisites

- Node.js 20
- pnpm 10
- Rust stable, compatible with the workspace `rust-version`
- Platform-specific Tauri dependencies
- macOS: a complete Xcode installation for native audio dependencies and release builds
- Windows: Visual Studio Build Tools with the Desktop C++ workload

## Development

From this directory:

```bash
pnpm install
pnpm check:i18n
pnpm check:network-boundary
pnpm tauri:dev
```

Build the frontend without packaging the desktop app:

```bash
pnpm build
```

Build the desktop bundle:

```bash
pnpm tauri:build
```

The `tauri:dev` and `tauri:build` scripts use `scripts/tauri-auto.js` to select the platform build path and prepare the `llama-helper` sidecar.

## Structure

```text
frontend/
  src/                 Next.js UI, hooks, services, and i18n resources
  src-tauri/           Rust/Tauri core, audio, ASR, diarization, and storage
  public/              Bundled frontend assets
  scripts/             Build and validation helpers
```

The current app does not require the archived FastAPI server under `backend/`. The frontend communicates with Rust through Tauri commands and events.

## Important checks

- Run `pnpm check:i18n` whenever user-facing copy or locale resources change.
- Run `pnpm check:network-boundary` when startup behavior or network-capable commands change.
- Run `pnpm build` for frontend type and production-build validation.
- Validate audio, ASR, speaker, and Provider changes in the packaged or development Tauri app.
- External summary Providers are optional; local recording and transcription must remain usable without them.
- Diagnostic logs remain local, rotate automatically, and are exported only after a user action in Settings.

See the repository [README](../README.md), [Roadmap](../ROADMAP.md), and [project agent rules](../AGENTS.md) for current product and engineering boundaries.
