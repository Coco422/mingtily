<div align="center">
  <img src="frontend/public/icon_128x128.png" width="96" alt="Mingtily logo" />
  <h1>Mingtily</h1>
  <p>Local-first AI meeting recording, transcription, speaker labels, and summaries.</p>
</div>

> Mingtily is an independent community fork of Meetily. It is not affiliated with or endorsed by the Meetily project.

Mingtily keeps recordings, transcripts, recovery data, and local models on your device by default. External LLM providers remain available as an explicit user choice because they can provide better summary quality and performance than local models on some hardware.

## Current capabilities

- Record microphone and system audio on desktop.
- Import common audio containers, including Opus audio inside M4A/MP4.
- Transcribe locally with Parakeet or Whisper.
- Apply offline speaker diarization and stable speaker labels.
- Generate summaries with an optional built-in model, Ollama, or a configured external provider.
- Store meetings, transcripts, templates, and recovery data in the Mingtily application data space.

## Privacy and network boundary

- Mingtily contains no usage analytics or telemetry client.
- Mingtily contains no background application updater.
- A normal cold start does not intentionally contact non-loopback services.
- Local model downloads begin only after a user click.
- External LLM requests happen only when the user configures and invokes that provider, including optional automatic summaries.
- Localhost discovery for services such as Ollama is allowed and does not leave the device.

See [PRIVACY_POLICY.md](PRIVACY_POLICY.md) for the detailed boundary.

## Development

Prerequisites: Node.js, pnpm, Rust, Tauri system dependencies, and platform audio/build tools.

```bash
cd frontend
pnpm install
pnpm tauri:dev
```

Build the desktop bundle:

```bash
cd frontend
pnpm tauri:build
```

Platform-specific build notes remain under [`docs/`](docs/).

## Model downloads

The application does not bundle large model weights. Downloads are user initiated and stored under the Mingtily application data directory.

Parakeet v3 is downloaded from the fixed revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce` of [`istupakov/parakeet-tdt-0.6b-v3-onnx`](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx), with built-in SHA256 verification.

## License and attribution

Mingtily is released under the MIT License. The original Meetily copyright notice is preserved in [LICENSE.md](LICENSE.md).

Model weights and third-party components may use different licenses. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) before redistribution.
