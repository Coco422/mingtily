# Mingtily Project Rules

This file contains repository-specific guidance for coding agents and contributors. Keep it focused on durable constraints and verified commands; use Git history and `ROADMAP.md` for project history and future plans.

## Project scope

Mingtily is a local-first desktop meeting recorder built with Tauri 2, Rust, Next.js 14, React 18, and TypeScript. The supported application lives under `frontend/`, with native code in `frontend/src-tauri/` and the built-in LLM sidecar in `llama-helper/`.

The Python/FastAPI code under `backend/` is a legacy upstream archive. Do not add supported features, deployment instructions, or new integrations there. Current frontend-to-native communication uses Tauri commands and events rather than a separate HTTP backend.

## Product and privacy boundaries

- Do not add telemetry, usage analytics, advertising identifiers, or background update checks.
- Cold start and ordinary local use must not initiate non-loopback requests. Explicit localhost discovery is allowed.
- Model downloads must be initiated by the user and must retain integrity verification.
- External LLM providers are supported, but only after explicit user configuration or action. Clearly disclose when transcript content will leave the device.
- Do not restore Meetily PRO, subscription, upstream marketing, or affiliation language. Preserve the independent-fork statement in public-facing legal/about surfaces.
- Do not reintroduce Meetily private services, updater infrastructure, licensing endpoints, or signing secrets.

## Architecture boundaries

- Tauri command registration and application startup: `frontend/src-tauri/src/lib.rs`.
- Audio capture, mixing, VAD, import, and recording: `frontend/src-tauri/src/audio/`.
- ASR provider abstraction: `frontend/src-tauri/src/audio/transcription/`.
- Local Whisper and Parakeet engines: `frontend/src-tauri/src/whisper_engine/` and `frontend/src-tauri/src/parakeet_engine/`.
- Speaker diarization: `frontend/src-tauri/src/speaker_diarization/`.
- Summary providers and built-in inference: `frontend/src-tauri/src/summary/` and `llama-helper/`.
- Frontend capability configuration: `frontend/src/services/capabilityConfigService.ts`.
- UI localization: `frontend/src/i18n/`.
- Local rotating logs and user-initiated diagnostic export: `frontend/src-tauri/src/diagnostic_logs.rs`.

Keep model assets and runtime selection separate:

- Models screens download, repair, inspect, and delete local assets.
- Services screens choose the Provider and model used for transcription, speaker diarization, and summaries.
- Model download completion must not silently switch the active Provider or model.
- Recording readiness checks must validate the configured transcription Provider; never hardcode Parakeet or Whisper as the universal prerequisite.
- Streaming ASR must use an explicit session and revision event contract rather than pretending partial hypotheses are ordinary `transcribe(audio)` results.
- Provisional streaming hypotheses are display-only. Persist only finalized transcript segments in meeting storage, recovery state, and transcript JSON.

Speaker diarization is optional and must fail open: ASR and recording continue without speaker labels when the feature is disabled, missing, damaged, or fails at runtime.

## Data and compatibility

- Application identifier: `com.mingcheng.mingtily`.
- Mingtily intentionally uses its own application data space and does not automatically migrate Meetily data.
- Meeting data is stored in SQLite; recovery and transient frontend state also use local storage/IndexedDB where already implemented.
- Preserve compatibility with existing meeting records and transcript JSON unless a task explicitly authorizes a migration.
- Do not change JobManager, cancellation behavior, VAD parameters, or audio mixing as part of unrelated work.

## Internationalization

- All reachable user-facing strings must use `i18next`/`react-i18next` resources.
- Supported UI locales are `en-US` and `zh-CN`; UI locale is independent from transcription and summary language.
- Model names, Provider IDs, stored speaker IDs such as `speaker_00`, and user transcript content are not translated.
- Keep translations natural and short enough for narrow dialogs and controls.
- Run `pnpm check:i18n` after changing UI copy or locale resources.

## Verified commands

Run frontend commands from `frontend/`:

```bash
pnpm install
pnpm check:i18n
pnpm check:network-boundary
pnpm build
pnpm tauri:dev
pnpm tauri:build
```

Run Rust workspace commands from the repository root:

```bash
cargo check --workspace
cargo test --workspace
cargo build --release -p llama-helper
```

On macOS, native dependencies such as `cidre` may require a complete Xcode installation and selected developer directory; Command Line Tools alone are not sufficient evidence that the release app can build.

## Validation expectations

- UI-only changes: run i18n validation and the Next.js production build.
- Rust changes: run targeted tests plus the broadest workspace check supported by the host environment.
- Audio, model, or Provider changes: verify the affected real application flow, not only compilation.
- Release candidates: verify the packaged app starts, is not blank, reports the expected version, and passes platform signature checks when signing is enabled.
- If a required platform check cannot run, report the missing coverage explicitly.

## GitHub Actions and release safety

- Unsigned CI builds must work without repository secrets.
- Pull-request validation must never require signing credentials.
- macOS sidecars and the Tauri bundle must be built for the same target architecture.
- Never print credentials or credential fragments in workflow logs.
- Current workflows are unsigned and must not reference signing secrets. Signing and notarization should be designed later using Mingtily-owned credentials and a reviewed draft-release process.

## Documentation

- Canonical public documents are `README.md`, `ROADMAP.md`, `PRIVACY_POLICY.md`, `THIRD_PARTY_NOTICES.md`, `CONTRIBUTING.md`, and `LICENSE.md`.
- Do not restore deleted upstream Meetily screenshots or documentation as Mingtily documentation.
- New user and developer documentation must be written from current Mingtily behavior and verified build procedures.
- Keep this file under roughly 300 lines and free of changelog-style narration.

## Workspace safety

- Preserve user changes and unrelated dirty files.
- Use `rg`/`rg --files` for repository search.
- Do not commit, push, tag, rebase, or stage changes unless the user explicitly requests it.
- Do not expose API keys, signing credentials, meeting content, or other sensitive local data in source, logs, tests, or responses.
