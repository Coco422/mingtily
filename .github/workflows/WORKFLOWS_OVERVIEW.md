# GitHub Actions workflows

Mingtily's pull-request and development workflows remain secret-free and unsigned. Tagged releases add integrity-signed Tauri updater artifacts; operating-system code signing and Apple notarization remain deferred.

## Automatic validation

### `ci.yml`

Runs on pull requests, pushes to `main` or `codex/**`, and manual dispatch.

- Installs frontend dependencies with the lockfile.
- Checks `en-US` and `zh-CN` resource parity.
- Verifies that local builds use the isolated Mingtily Dev identity while tagged releases restore the production identity.
- Audits cold-start source entry points for implicit remote-network calls.
- Builds the Next.js frontend.
- Runs `cargo fmt --check`, Rust tests, and `cargo check --all-targets` on Ubuntu 22.04. Linux VAD execution tests are temporarily skipped because the monolithic test binary links two ONNX Runtime implementations; they continue to compile and are covered on the primary macOS development platform.
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

A maintainer must create a draft release with their authenticated GitHub account after Validation succeeds on `main`. Creating the draft for a new `vX.Y.Z` tag also creates that tag and triggers this workflow; do not push the tag first. The workflow rejects a missing, published, prerelease, or bot-authored draft, then builds macOS Apple Silicon and Windows x64 bundles, uploads installers plus signed updater archives and `latest.json`, and publishes only after both platforms succeed. Keeping the draft author as a maintainer lets GitHub attribute the published `ReleaseEvent` to that maintainer instead of `github-actions[bot]`.

From a clean, up-to-date `main` checkout, use release notes prepared outside the repository:

```bash
version=0.7.4
gh release create "v${version}" \
  --draft \
  --target "$(git rev-parse HEAD)" \
  --title "Mingtily v${version}" \
  --notes-file /path/to/release-notes.md
```

Linux remains available through unsigned development workflows until its Silero/ORT and static Sherpa-ONNX runtime conflict is resolved. The repository secret `TAURI_SIGNING_PRIVATE_KEY` signs updater payloads; it is unrelated to Apple or Windows platform signing.

The base Tauri configuration is deliberately isolated as `Mingtily Dev` / `com.mingcheng.mingtily.dev`. `release.yml` is the only supported path that overlays `tauri.release.conf.json` to produce the production `Mingtily` / `com.mingcheng.mingtily` application.

## Recommended workflow

1. Let `ci.yml` validate every pull request.
2. Let `build-linux.yml` validate the unsigned Linux bundle and cold-start network boundary.
3. Use `build-devtest.yml` or a standalone platform workflow when a downloadable development artifact is needed.
4. After Validation succeeds on `main`, create the maintainer-authored draft release as shown above; its tag triggers the workflow that publishes the GitHub Release used by the opt-in updater.
