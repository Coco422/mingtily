# DevTest builds

`build-devtest.yml` creates development artifacts for macOS Apple Silicon, Windows x64, Ubuntu 22.04, and Ubuntu 24.04. It is manually triggered from the GitHub Actions page.

## Current policy

- All DevTest artifacts are unsigned and require no repository secrets.
- Uploading artifacts is enabled by default with 30-day retention.
- Signing and notarization are not implemented in the current workflows and are not a 0.5.x acceptance requirement.

## Outputs

| Platform | Target | Expected bundles |
|---|---|---|
| macOS | `aarch64-apple-darwin` | `.app`, `.dmg` |
| Windows | `x86_64-pc-windows-msvc` | `.msi`, NSIS `.exe` |
| Ubuntu 22.04 | `x86_64-unknown-linux-gnu` | `.deb` |
| Ubuntu 24.04 | `x86_64-unknown-linux-gnu` | `.AppImage`, `.rpm` |

The `llama-helper` sidecar and Tauri application must use the same target architecture. A successful bundle build does not replace real microphone, system-audio, ASR, or installer testing on that platform.

## Running a build

1. Open **Actions → Build and Test - DevTest**.
2. Select the branch.
3. Keep artifact upload enabled when another machine needs to test the build.
4. Review each matrix job independently; one platform can fail while others finish.

For routine pull requests, prefer `ci.yml` and the automatic Linux build. Use DevTest when platform bundles are actually needed.
