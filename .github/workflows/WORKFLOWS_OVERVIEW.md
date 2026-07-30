# GitHub Actions workflows

Mingtily's pull-request and development workflows remain secret-free and unsigned. Tagged releases add integrity-signed Tauri updater artifacts; operating-system code signing and Apple notarization remain deferred.

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

## Tagged releases

### `release.yml`

Pushing a matching `vX.Y.Z` tag builds macOS Apple Silicon, Windows x64, and Linux x64 bundles. The workflow creates a draft release, uploads installers plus signed updater archives and `latest.json`, then publishes only after every platform succeeds. The repository secret `TAURI_SIGNING_PRIVATE_KEY` signs updater payloads; it is unrelated to Apple or Windows platform signing.

## Recommended workflow

1. Let `ci.yml` validate every pull request.
2. Let `build-linux.yml` validate the unsigned Linux bundle and cold-start network boundary.
3. Use `build-devtest.yml` or a standalone platform workflow when a downloadable development artifact is needed.
4. Push a matching version tag only after Validation succeeds on `main`; the tag workflow publishes the GitHub Release used by the opt-in updater.
