# Mingtily Privacy Policy

Last updated: July 24, 2026

## Local-first data handling

Mingtily stores meeting audio, transcripts, speaker labels, summaries, recovery data, settings, templates, and downloaded local models on the user's device. Mingtily does not operate a hosted account or meeting-storage service.

Mingtily does not include usage analytics, telemetry clients, advertising identifiers, or a background application updater.

## Network activity

A normal application cold start is designed not to contact non-loopback services. Mingtily may probe explicitly supported localhost services such as Ollama; loopback traffic remains on the device.

Non-loopback network requests occur only after a user action or configuration that requires them:

- Downloading an ASR, punctuation restoration, speaker diarization, Whisper, or built-in summary model.
- Using an external summary provider such as OpenAI, Anthropic, Groq, OpenRouter, or a custom OpenAI-compatible endpoint.
- Opening an external link selected by the user.

Model downloads are never started automatically during onboarding or application startup.

## External LLM providers

External providers are optional. When a user selects an external provider and generates a summary, the relevant transcript content and prompt are sent to the configured service. That provider's privacy policy, retention rules, location, and account settings apply.

Mingtily does not proxy those requests through a Mingtily-operated service. API credentials are stored locally by the application and are sent only to the configured provider endpoint when required.

Use Ollama or a downloaded built-in model when transcript content must remain on the device.

## User control

Users can inspect, export, or delete local meeting data through the application and the operating system's application data directory. Uninstalling the application may not automatically remove user-created recordings or application data; remove those directories manually if complete deletion is required.

## Diagnostic logs

Mingtily keeps a small rotating set of diagnostic logs on the device to investigate startup, recording, model, and Provider failures. The default retention is five files of up to 5 MB each. Logs are not telemetry and are never uploaded automatically.

A diagnostic export is created only after the user selects **Export diagnostics** in Settings and chooses a destination. The export replaces the user's home-directory path and removes obvious credential-bearing lines. It may still include device names, model names, filenames, timestamps, and technical error context, so users should review the file before sharing it.

## Security scope

Local data is protected by the device's operating-system account, filesystem permissions, and any disk encryption enabled by the user. Mingtily does not claim to add independent at-rest encryption to every stored artifact.

## Open-source transparency

The source code is available for review. Third-party components and model weights have their own licenses and terms; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Mingtily is an independent community fork of Meetily. It is not affiliated with or endorsed by the Meetily project.

Privacy questions and security reports can be filed in the Mingtily GitHub repository.
