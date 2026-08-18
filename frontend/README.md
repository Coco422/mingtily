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
pnpm check:app-identity
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

Normal local and development builds are named **Mingtily Dev** and use `com.mingcheng.mingtily.dev`, keeping their database, models, logs, permissions, and WebView storage separate from an installed production app. They keep updater-artifact generation off and require no signing secret. Tagged releases overlay `src-tauri/tauri.release.conf.json`, restore the production `Mingtily` / `com.mingcheng.mingtily` identity, use the repository's Tauri updater signing secret, and publish `latest.json` through GitHub Actions.

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

## State and data contracts

- `src/contexts/SummaryJobsContext.tsx` owns summary polling, stream snapshots, terminal notifications, and unread state across route changes. SQLite summary-process rows remain the source of truth; unmounting meeting details must not cancel a running job.
- `src-tauri/src/speaker_mapping.rs` and `src-tauri/migrations/20260817000000_add_meeting_speaker_maps.sql` store revisioned, meeting-local participant mappings. The resolver overlays names and colors without rewriting historical transcript JSON.
- `src-tauri/src/audio/transcription/terminology.rs` wraps the configured finalized transcription Provider. Exact replacements apply once to finalized text; provisional streaming hypotheses are never corrected or persisted.
- Relevant Tauri contracts are `api_get_summary` / `api_process_transcript` / `api_cancel_summary`, `api_get_meeting_speaker_map` / `api_save_meeting_speaker_map`, and `terminology_get_config` / `terminology_save_config`. Speaker-map saves require the expected revision and reject stale writes.
- Retranscription clears a meeting's speaker mapping only in the successful database transaction. Deleting a meeting explicitly removes the corresponding mapping.

## Important checks

- Run `pnpm check:i18n` whenever user-facing copy or locale resources change.
- Run `pnpm check:app-identity` after changing Tauri configuration or packaging commands. Local commands must remain isolated from production data, while tagged releases must retain the canonical production identity.
- Run `pnpm check:network-boundary` when startup behavior or network-capable commands change.
- Run `pnpm build` for frontend type and production-build validation.
- Validate audio, ASR, speaker, and Provider changes in the packaged or development Tauri app.
- Validate background-summary navigation, speaker-map revision conflicts, and terminology corrections through their affected application flows when those contracts change.
- Database migrations must retain published checksums and include an upgrade regression test from the latest public baseline. A database created by a newer app version must produce an actionable startup error instead of being reset or opened by an older schema.
- External summary Providers are optional; local recording and transcription must remain usable without them.
- Diagnostic logs remain local, rotate automatically, and are exported only after a user action in Settings.
- Automatic GitHub Release checks are disabled by default and run only after the user enables them.

See the repository [README](../README.md), [Roadmap](../ROADMAP.md), and [project agent rules](../AGENTS.md) for current product and engineering boundaries.
