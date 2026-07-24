# GitHub Actions workflows

Mingtily's active development workflows are secret-free and unsigned. Code signing, notarization, and production release automation are intentionally absent until the project has Mingtily-owned credentials and a concrete distribution need.

## Automatic validation

### `ci.yml`

Runs on pull requests, pushes to `main` or `codex/**`, and manual dispatch.

- Installs frontend dependencies with the lockfile.
- Checks `en-US` and `zh-CN` resource parity.
- Audits cold-start source entry points for implicit remote-network calls.
- Builds the Next.js frontend.
- Runs `cargo fmt --check`, Rust tests, and `cargo check --all-targets` on Ubuntu 22.04.
- Requires no repository secrets.

### `build-linux.yml`

Runs automatically on relevant pull requests and can also be started manually.

- Pull requests build an unsigned debug DEB on Ubuntu 22.04.
- The built application is launched under Xvfb and `strace`; any non-loopback cold-start connection fails the job.
- Manual runs can build release DEB, AppImage, RPM, or all applicable formats on Ubuntu 22.04/24.04.
- Manual artifacts are retained for 30 days.
- Requires no signing secrets.

## Manual development builds

### `build-devtest.yml`

Builds macOS Apple Silicon, Windows x64, and Linux x64 artifacts in parallel through the reusable unsigned workflow.

### `build-macos.yml`

Builds an unsigned Apple Silicon `.app` and `.dmg`. It has no certificate or notarization inputs.

### `build-windows.yml`

Builds unsigned Windows x64 MSI/NSIS artifacts. It has no DigiCert or certificate inputs.

### `build.yml`

Reusable unsigned build implementation shared by DevTest and the standalone macOS and Windows workflows. It builds the `llama-helper` sidecar for the same target as the Tauri application and uploads artifacts when requested.

### `pr-main-check.yml`

Manual semantic-version and branch summary. It does not build the application.

## Deferred release automation

There is currently no production release workflow. A future implementation must start from Mingtily-owned credentials, keep pull-request builds secret-free, create a draft release first, and be verified on clean machines before publication.

## Recommended workflow

1. Let `ci.yml` validate every pull request.
2. Let `build-linux.yml` validate the unsigned Linux bundle and cold-start network boundary.
3. Use `build-devtest.yml` or a standalone platform workflow when a downloadable development artifact is needed.
4. Record platform and hardware limitations in the pull request; unsigned CI artifacts are development builds, not production releases.
