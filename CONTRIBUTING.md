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

These commands run **Mingtily Dev** with the isolated identifier `com.mingcheng.mingtily.dev`. Never change local development commands to use the production `com.mingcheng.mingtily` data directory. Tagged releases apply the production identity through `frontend/src-tauri/tauri.release.conf.json`.

Use a feature branch based on the repository's current default branch:

```bash
git switch -c feature/short-description
```

## Validation

Run the checks relevant to your change. Frontend and translation changes should normally include:

```bash
cd frontend
pnpm check:i18n
pnpm check:app-identity
pnpm check:network-boundary
pnpm build
```

Rust changes should include targeted tests and, when the host platform supports all native dependencies:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
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
- Do not add telemetry, Meetily marketing, or implicit remote requests. Automatic release checks must remain opt-in, disclosed, and pointed at Mingtily's public GitHub Releases.
- New UI strings must support `en-US` and `zh-CN`.
- New ASR and speaker models should use the shared model manifest and management architecture.
- The Python/FastAPI code under `backend/` is an unsupported legacy archive.

## Commit messages

Prefer focused commits using this format:

```text
<type>(scope): summary
```

Common types include `feat`, `fix`, `docs`, `refactor`, `test`, `build`, and `chore`.

## Maintainer releases

After the release commit is on `main` and Validation succeeds, create the draft release with an authenticated maintainer account before pushing the version tag. GitHub may keep a new draft tag virtual, so create and push the annotated tag only after the draft exists; that tag push triggers the Release workflow.

```bash
version="$(node -p "require('./frontend/package.json').version")"
gh release create "v${version}" \
  --draft \
  --target "$(git rev-parse HEAD)" \
  --title "Mingtily v${version}" \
  --notes-file /path/to/release-notes.md
git tag -a "v${version}" -m "Mingtily v${version}"
git push origin "v${version}"
```

The workflow verifies that the draft is stable and maintainer-authored, builds both supported release targets, validates updater signatures and `latest.json`, and only then publishes it. This preserves the maintainer identity on GitHub's release event while keeping artifact publication gated on successful builds.

By contributing, you agree that your contribution is licensed under the repository's MIT License.
