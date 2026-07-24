<div align="center">
  <img src="frontend/public/icon_128x128.png" width="96" alt="Mingtily logo" />
  <h1>Mingtily</h1>
  <p>Local-first meeting recording, transcription, speaker labels, and AI summaries.</p>
</div>

> Mingtily is an independent community fork of Meetily. It is not affiliated with or endorsed by the Meetily project.

Mingtily is a desktop meeting assistant for people who want recordings, transcripts, speaker labels, and local models to stay on their own device. External LLM Providers remain available as an explicit choice when cloud models provide better summary quality or performance.

Current version: **0.6.0**. The project is usable for development and personal testing. GitHub Actions can produce unsigned development installers; signed public releases are not yet provided.

## Highlights

- Record microphone and system audio in one meeting timeline.
- Import common audio formats, including Opus audio inside M4A/MP4 containers.
- Transcribe locally with Whisper, NVIDIA Parakeet, SenseVoice, Paraformer, or Qwen3-ASR.
- Add speaker labels with Sherpa ONNX, Pyannote segmentation, and 3D-Speaker ERes2Net embeddings.
- Refine provisional live speaker labels after recording stops without running ASR again.
- Generate summaries with an optional built-in model, Ollama, or a user-configured external Provider.
- Use the interface in `zh-CN` or `en-US`; UI, transcription, and summary languages are configured independently.
- Recover audio checkpoints and transcript state after an interrupted recording.

## Privacy and network boundary

Mingtily has no telemetry client, usage analytics, advertising identifier, background updater, or Mingtily-hosted account service.

- Cold start and ordinary local use are designed not to contact non-loopback services.
- Localhost discovery, such as checking an explicitly configured Ollama endpoint, stays on the device.
- Model downloads start only after a user action.
- External LLM requests occur only after the user configures or invokes that Provider. The relevant transcript content leaves the device in that case.
- Diagnostic logs rotate locally at five files of up to 5 MB each. They are never uploaded automatically and can be exported only from Settings after an explicit user action.
- Diagnostic exports replace the user's home-directory path and obvious credential-bearing log lines. Review an export before sharing it because filenames, device names, and error context may still be useful to diagnosis.

See [PRIVACY_POLICY.md](PRIVACY_POLICY.md) for the full boundary.

## Models and Providers

Large model weights are not bundled with the application. Models are downloaded into Mingtily's application-data directory only when requested.

| Capability | Current choices | Notes |
|---|---|---|
| Speech recognition | Whisper, Parakeet TDT 0.6B v2/v3, SenseVoice Small int8, Paraformer Small int8, Qwen3-ASR 0.6B int8 | SenseVoice is the recommended Chinese choice and supports forced Mandarin or Cantonese. Paraformer is the lightweight Chinese/English choice. Qwen3-ASR is a larger multilingual Beta option. |
| Speaker diarization | Sherpa ONNX `sherpa-v1` | Pyannote segmentation 3.0 int8 plus 3D-Speaker ERes2Net; approximately 47 MB to download. |
| Built-in summaries | Qwen 3.5 2B/4B, Gemma 3 1B/4B | Optional GGUF downloads; local inference uses the bundled `llama-helper` sidecar. |
| Local service summaries | Ollama | Defaults to a loopback endpoint and can use models already managed by Ollama. |
| External summaries | OpenAI, Anthropic, Groq, OpenRouter, OpenAI Compatible | Transcript content is sent directly to the configured endpoint. |

Parakeet v3 is pinned to revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce` of [`istupakov/parakeet-tdt-0.6b-v3-onnx`](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) and verified with built-in SHA256 values.

## Platform status

| Platform | Status | Current scope |
|---|---|---|
| macOS Apple Silicon | Primary development platform | Local development and unsigned builds; microphone, system audio, Metal Whisper, import, speaker diarization, and summaries. |
| Windows x64 | CI/dev-test target | Unsigned MSI/NSIS build path exists; audio and installer behavior still need broader hardware testing. |
| Linux x64 | CI/dev-test target | Pull requests build an unsigned DEB and run a cold-start loopback-network smoke test; desktop audio varies by distribution. |

Code signing, Apple notarization, and production installer signing are deferred until the project has appropriate Mingtily-owned credentials and community demand.

## Using the application

1. Open **Settings → Models** and download a local ASR model.
2. Open **Settings → Services** and select the installed transcription model.
3. Optionally download and enable the speaker-diarization model.
4. Choose the transcription language. For predictable Chinese output, use SenseVoice and choose Mandarin or Cantonese; Whisper remains available for broader language coverage and translation to English.
5. Start a recording or enable the Beta import/retranscription feature.
6. Configure Built-in AI, Ollama, or an external summary Provider only if summaries are needed.

Model downloads do not silently change the active Provider or model.

## Local development

Prerequisites:

- Node.js 20
- pnpm 10
- Rust stable
- Tauri 2 platform prerequisites
- A complete Xcode installation on macOS when native crates require Apple frameworks

Clone and run:

```bash
git clone https://github.com/Coco422/mingtily.git
cd mingtily/frontend
pnpm install --frozen-lockfile
pnpm tauri:dev
```

Build an unsigned desktop bundle:

```bash
cd frontend
pnpm tauri:build
```

## GitHub Actions

- `Validation` runs frontend, i18n, network-boundary, formatting, Rust test, and Rust check gates.
- `Build and Test - Linux` also builds an unsigned Linux package on relevant pull requests and checks cold-start network connections under `strace`.
- The manual `Build and Test - DevTest`, macOS, Windows, and Linux workflows produce unsigned development artifacts without repository secrets.
- Signing, notarization, and production release automation are intentionally absent until Mingtily owns the required credentials and has a concrete distribution need.

On Linux, install the WebKitGTK, app-indicator, ALSA, X11, and packaging dependencies shown in [`.github/workflows/build-linux.yml`](.github/workflows/build-linux.yml). See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for platform setup.

## Validation

Frontend and boundary checks:

```bash
cd frontend
pnpm check:i18n
pnpm check:network-boundary
pnpm build
```

Rust checks from the repository root:

```bash
cargo fmt --all -- --check
cargo test --workspace --lib --bins
cargo check --workspace --all-targets
cargo build --release -p llama-helper
```

Real audio integration tests do not commit private fixtures. Point the ignored harnesses at local files:

```bash
TEST_AUDIO_PATH=/path/to/audio.wav cargo test test_import_pipeline_decode_vad -- --ignored --nocapture
TEST_OPUS_M4A_PATH=/path/to/audio.m4a cargo test test_opus_m4a_decode -- --ignored --nocapture
```

## Repository layout

- `frontend/`: supported Next.js UI and Tauri application.
- `frontend/src-tauri/`: Rust audio, ASR, diarization, persistence, diagnostics, and Provider commands.
- `llama-helper/`: built-in local summary sidecar.
- `backend/`: unsupported upstream Python/FastAPI archive; do not add new Mingtily features there.
- `.github/workflows/`: secret-free validation and manual platform build workflows.

See [AGENTS.md](AGENTS.md) for durable engineering boundaries and [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## Known limitations

- Parakeet and the current offline Paraformer model use automatic language detection. SenseVoice, Qwen3-ASR, and Whisper accept supported fixed-language hints.
- Current recording text still appears after each VAD speech segment completes. Online Paraformer partial hypotheses require a separate streaming session and revision protocol planned for 0.7.
- Live speaker labels appear after a VAD segment finishes; they are not token-level labels.
- Overlapping speech is assigned to the dominant speaker and is not transcribed twice.
- Speaker names cannot yet be renamed and are not remembered across meetings.
- Platform installers are currently unsigned development artifacts.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for Chinese ASR, streaming Paraformer, local and domestic Provider presets, MCP, meeting RAG, and offline knowledge workflows.

## License and attribution

Mingtily is released under the MIT License. The original Meetily copyright notice remains in [LICENSE.md](LICENSE.md).

Model weights and third-party components may use different licenses. Review [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) before redistribution.
