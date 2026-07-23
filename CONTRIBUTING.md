# Contributing to Mingtily

Thank you for helping improve Mingtily. The project welcomes reproducible bug reports, focused fixes, translation improvements, model performance data, and cross-platform build work.

## Before starting

- Search existing issues and pull requests.
- Keep changes focused on one problem or capability.
- For larger features, open an issue describing the user problem, privacy impact, model/runtime requirements, and proposed acceptance criteria.
- Read [AGENTS.md](AGENTS.md) for repository boundaries and verified commands.
- Read [ROADMAP.md](ROADMAP.md) before adding a new model or Provider.

## Local development

```bash
git clone https://github.com/Coco422/mingtily.git
cd mingtily/frontend
pnpm install
pnpm tauri:dev
```

Use a feature branch based on the repository's current default branch:

```bash
git switch -c feature/short-description
```

## Validation

Run the checks relevant to your change. Frontend and translation changes should normally include:

```bash
cd frontend
pnpm check:i18n
pnpm build
```

Rust changes should include targeted tests and, when the host platform supports all native dependencies:

```bash
cargo check --workspace
cargo test --workspace
```

Audio, model, Provider, packaging, and persistence changes should also be verified through the affected application flow. Explain any check you could not run.

## Pull requests

- Describe the user-visible outcome and technical approach.
- Link related issues where applicable.
- Include testing evidence and platform details.
- Add screenshots for meaningful UI changes in both supported languages when text length or layout is affected.
- Update public documentation when behavior, privacy boundaries, configuration, or build requirements change.
- Do not include model weights, recordings, API keys, signing credentials, generated build output, or local application data.

## Project boundaries

- Mingtily is local-first but supports user-configured external LLM providers.
- Do not add telemetry, background updating, Meetily marketing, or implicit remote requests.
- New UI strings must support `en-US` and `zh-CN`.
- New ASR and speaker models should use the shared model manifest and management architecture.
- The Python/FastAPI code under `backend/` is an unsupported legacy archive.

## Commit messages

Prefer focused commits using this format:

```text
<type>(scope): summary
```

Common types include `feat`, `fix`, `docs`, `refactor`, `test`, `build`, and `chore`.

By contributing, you agree that your contribution is licensed under the repository's MIT License.
