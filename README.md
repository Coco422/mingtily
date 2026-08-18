<div align="center">
  <img src="frontend/public/icon_128x128.png" width="96" alt="Mingtily app icon" />
  <h1>Mingtily</h1>
  <p>本地优先的会议录音、转写、说话人整理与 AI 总结工具。<br />A local-first meeting recorder, transcription, speaker organization, and AI summary app.</p>
  <p><a href="#readme-zh">中文</a> · <a href="#readme-en">English</a></p>
</div>

> **中文：** Mingtily 是 Meetily 的独立社区分支，与 Meetily 项目不存在隶属、合作或背书关系。<br />
> **English:** Mingtily is an independent community fork of Meetily. It is not affiliated with or endorsed by the Meetily project.

<a id="readme-zh"></a>

## 中文

Mingtily 是一款本地优先的桌面会议助手，面向希望将录音、转写、说话人标签和本地模型保留在自己设备上的个人、开发者与小团队。需要更高总结质量或性能时，也可以由用户主动配置并调用外部 LLM Provider。

当前版本：**[0.7.5](https://github.com/Coco422/mingtily/releases/tag/v0.7.5)**。带版本标签的 GitHub Actions 会发布 macOS Apple Silicon 与 Windows x64 开发版。安装包暂未进行操作系统级签名，但 Tauri 更新包由 Mingtily 完整性签名。Linux 目前作为无签名手动开发构建目标。

### 主要能力

- 在同一会议时间线中录制麦克风与系统音频。
- 导入常见音频格式，包括 M4A/MP4 容器中的 Opus 音频。
- 使用 Whisper、NVIDIA Parakeet、SenseVoice、离线或流式 Paraformer、Qwen3-ASR 与 FunASR Nano 在本地转写。
- 使用 Sherpa ONNX、Pyannote segmentation 与 3D-Speaker ERes2Net 添加说话人标签；说话人分离不可用时，录音和转写仍会继续。
- 在单场会议内命名、合并或拆分检测到的说话人标签；显示、颜色、复制文本和后续摘要输入统一使用整理后的姓名。
- 使用全局自定义术语、精确纠错和可选的高级 FST 兼容规则改善最终转写；临时流式文本保持原样。
- 使用可选的内置模型、Ollama 或用户配置的外部 Provider 生成 AI 总结。
- 摘要在应用级后台任务中继续生成；离开会议详情再返回时会恢复进度、结果或持久错误，而不是显示空白页面。
- 流式展示 AI 总结；首个 token 前显示真实处理阶段和累计耗时，支持的 reasoning 标签会在生成时显示，完成后折叠且不会写入最终摘要。
- 展示录音时长和停止后的分阶段处理进度，避免长时间本地处理没有反馈。
- 使用 `zh-CN` 或 `en-US` 界面；界面语言、转写语言和总结语言相互独立。
- 在录音被意外中断后恢复音频检查点和转写状态。

### 隐私与网络边界

Mingtily 不包含遥测客户端、使用分析、广告标识符或 Mingtily 托管的账户服务。自动更新检查默认关闭。

- 冷启动和普通本地使用被设计为不访问非 loopback 服务。
- 对用户明确配置的 Ollama 等 localhost 服务进行探测时，数据仍留在本机。
- 模型只会在用户主动操作后下载，下载完成不会静默切换当前 Provider 或模型。
- 用户可以手动检查 GitHub Releases，或在设置中明确开启自动检查。更新请求不包含会议或转写内容。
- 外部 LLM 请求只会在用户配置或调用相应 Provider 后发生；此时相关转写内容会离开设备并直接发送到所配置的端点。
- 诊断日志仅在本地滚动保存，最多五个文件、每个不超过 5 MB；只有用户在设置中主动导出时才会生成诊断包，应用不会自动上传。
- 诊断导出会替换用户主目录路径并过滤明显包含凭据的日志行。分享前仍应人工检查其中的文件名、设备名和错误上下文。

完整边界请参阅 [PRIVACY_POLICY.md](PRIVACY_POLICY.md)。

### 模型与 Provider

应用安装包不包含大型模型权重。模型仅在用户请求后下载到 Mingtily 的应用数据目录。

| 能力 | 当前选择 | 说明 |
|---|---|---|
| 语音识别 | Whisper、Parakeet TDT 0.6B v2/v3、SenseVoice Small int8、Paraformer Small int8、Paraformer Streaming zh/en int8、Qwen3-ASR 0.6B int8、FunASR Nano int8 | SenseVoice 是推荐的中文选择，支持强制普通话或粤语。Parakeet 仅支持英文。Beta 实时增强使用流式模型显示临时文本，再由独立最终模型保存正式转写。 |
| 标点恢复 | Sherpa ONNX CT-Transformer zh/en int8 | 可选的本地后处理，仅作用于 SenseVoice 最终片段；下载约 62 MiB，不可用时保留原始转写。 |
| 说话人分离 | Sherpa ONNX `sherpa-v1` | Pyannote segmentation 3.0 int8 与 3D-Speaker ERes2Net，下载约 47 MB。 |
| 内置总结 | Qwen 3.5 2B/4B、Gemma 3 1B/4B | 可选 GGUF 下载；本地推理由随应用提供的 `llama-helper` sidecar 执行。 |
| 本地服务总结 | Ollama | 默认使用 loopback 端点，可复用 Ollama 已管理的模型。 |
| 外部总结 | OpenAI、Anthropic、Groq、OpenRouter、OpenAI Compatible | 转写内容会直接发送到用户配置的端点。 |

全新安装默认选择 SenseVoice Small int8。下载约 158 MiB，安装后约 226 MiB；首次引导不会自动下载，必须由用户点击下载。

Parakeet 仅支持英文。Parakeet v3 固定到 [`istupakov/parakeet-tdt-0.6b-v3-onnx`](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) 的 revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce`，并使用内置 SHA-256 校验值验证。

### 平台状态

| 平台 | 状态 | 当前范围 |
|---|---|---|
| macOS Apple Silicon | 主要开发平台 | 发布无签名 DMG/app 与完整性签名更新包；支持麦克风、系统音频、Metal Whisper、导入、说话人分离和总结。 |
| Windows x64 | CI / 开发测试目标 | 发布无签名 MSI/NSIS 与更新包；仍需扩大真实硬件上的音频与安装器测试。 |
| Linux x64 | CI / 开发测试目标 | 提供无签名的手动 AppImage/DEB 构建与拉取请求冷启动网络检查。带标签的更新包暂停，等待 ONNX Runtime 冲突统一并重新验证录音。 |

Apple notarization、Apple Developer ID 与生产级 Windows 安装包签名暂缓，直到项目拥有合适的 Mingtily 自有凭据并出现明确社区需求。

### 使用应用

1. 打开 **设置 → 模型**，下载一个本地语音识别模型。
2. 打开 **设置 → 服务**，选择已安装的转写 Provider 与模型。
3. 使用 SenseVoice 时，可以下载可选的标点恢复模型来改善中英文最终转写。
4. 在 **设置 → 服务 → 自定义术语** 中维护姓名与领域术语，并按需添加“识别成 → 替换为”的精确纠错。Qwen3-ASR、FunASR Nano 与 Whisper 还会将术语作为模型提示；其他模型仍会对最终保存文本应用精确纠错。
5. Sherpa ONNX 用户如有兼容需要，可在折叠的高级选项中选择一个预生成 FST；应用不会生成 FST、下载 Pynini 或把多个规则传入原生层。
6. 如需连续修订的实时文字，可下载 **Paraformer Streaming zh/en int8**，在 **设置 → 服务** 中选择 **Beta · 实时增强**，并分别指定流式模型与最终模型。
7. 可选下载并启用说话人分离模型；已知参会人数时，可以在 **设置 → 服务** 中指定 1–10 人。
8. 开始录音，或启用 Beta 文件导入/重新转写。会议完成后可在详情页管理说话人，将多个检测标签合并为同一人物。
9. 仅在需要总结时配置 Built-in AI、Ollama 或外部 Provider。后台总结在页面切换后仍会继续，侧边栏会显示运行和未读终态。
10. 在 **设置 → 常规** 中手动检查更新，或明确开启自动检查。

会议级说话人映射不会自动改写旧摘要；需要时请重新生成摘要。模型下载也不会静默改变当前 Provider 或模型。

### 本地开发

前置依赖：Node.js 20、pnpm 10、Rust stable、Tauri 2 对应平台的系统依赖；macOS 原生 crate 需要 Apple framework 时必须安装完整 Xcode。

```bash
git clone https://github.com/Coco422/mingtily.git
cd mingtily/frontend
pnpm install --frozen-lockfile
pnpm tauri:dev
```

本地开发与测试默认运行 **Mingtily Dev**（`com.mingcheng.mingtily.dev`），使用独立的数据、日志、权限和 WebView 存储，不会读取或迁移已安装的正式 Mingtily 数据。

构建无签名的本地测试安装包：

```bash
cd frontend
pnpm tauri:build
```

该命令同样生成 Mingtily Dev。只有带版本标签的 Release 工作流会叠加生产配置，生成 `Mingtily`（`com.mingcheng.mingtily`）。

### 验证与仓库结构

在 `frontend/` 中运行 `pnpm check:app-identity`、`pnpm check:i18n`、`pnpm check:network-boundary` 和 `pnpm build`。在仓库根目录运行 `cargo fmt --all -- --check`、`cargo test --workspace --lib --bins`、`cargo check --workspace --all-targets` 和 `cargo build --release -p llama-helper`。

- `frontend/`：受支持的 Next.js UI 与 Tauri 应用。
- `frontend/src-tauri/`：Rust 音频、ASR、说话人分离、持久化、诊断和 Provider 命令。
- `llama-helper/`：内置本地总结 sidecar。
- `backend/`：不受支持的上游 Python/FastAPI 存档；请勿在其中添加新的 Mingtily 功能。
- `.github/workflows/`：无 secrets 的验证/手动构建，以及带标签的 GitHub Release 工作流。

提交代码前请阅读 [AGENTS.md](AGENTS.md) 与 [CONTRIBUTING.md](CONTRIBUTING.md)。

### 已知限制

- 一台 Windows x64 测试设备上，SenseVoice 安装完成后应用曾在刷新模型状态时退出。重启后模型仍可使用，相关 Windows 模块仍在排查。
- Parakeet 仅支持英文。FunASR Nano 和两种 Paraformer 自动检测语言；SenseVoice、Qwen3-ASR 与 Whisper 接受各自支持的固定语言提示。
- Beta 实时增强中的 Paraformer Streaming 临时文本会在说话时持续修订，且不会写入 SQLite、IndexedDB 或 transcript JSON。
- 说话人标签在一个 VAD 片段结束后出现，并非 token 级标签；重叠语音只分配给占主导的说话人，不会重复转写。
- 自动说话人检测属于启发式方法。已知会议人数时，建议指定 1–10 人来限制实时身份并辅助最终校正。
- 说话人姓名仅保存在当前会议中，不提供跨会议身份记忆、声纹实名识别或姓名搜索；重新转写成功后会清除该会议的映射。
- 自定义术语首版为全局配置，仅修改最终保存文本；不提供会议级覆盖，也不会对临时流式文本执行精确纠错。
- 平台安装包目前属于无操作系统签名的开发产物；只有 Tauri 更新包携带 Mingtily 完整性签名。

### 路线图、许可证与归属

后续 Provider 生态、流式性能、离线知识能力和 1.0 稳定性计划见 [ROADMAP.md](ROADMAP.md)。

Mingtily 使用 MIT License。原始 Meetily 版权声明保留在 [LICENSE.md](LICENSE.md) 中。模型权重与第三方组件可能采用不同许可证，重新分发前请阅读 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

<p align="right"><a href="#readme-en">Read in English →</a></p>

---

<a id="readme-en"></a>

## English

Mingtily is a local-first desktop meeting assistant for individuals, developers, and small teams that want recordings, transcripts, speaker labels, and local models to stay on their own devices. When better summary quality or performance is needed, users can explicitly configure and invoke an external LLM Provider.

Current version: **[0.7.5](https://github.com/Coco422/mingtily/releases/tag/v0.7.5)**. Tagged GitHub Actions releases target macOS Apple Silicon and Windows x64. Installers remain unsigned at the operating-system level, while Tauri updater payloads are integrity-signed by Mingtily. Linux is currently available as an unsigned manual development-build target.

### Highlights

- Record microphone and system audio in one meeting timeline.
- Import common audio formats, including Opus audio inside M4A/MP4 containers.
- Transcribe locally with Whisper, NVIDIA Parakeet, SenseVoice, offline or streaming Paraformer, Qwen3-ASR, and FunASR Nano.
- Add speaker labels with Sherpa ONNX, Pyannote segmentation, and 3D-Speaker ERes2Net. Recording and transcription continue if diarization is unavailable.
- Name, merge, or split detected speaker labels within a meeting. Display, color, copied text, and future summary input all use the resolved participant names.
- Improve finalized transcripts with global custom terminology, exact replacements, and optional advanced FST compatibility rules. Provisional streaming text remains unchanged.
- Generate summaries with an optional built-in model, Ollama, or a user-configured external Provider.
- Keep summaries running as application-level background jobs. Leaving and reopening meeting details restores progress, results, or persistent errors instead of showing a blank page.
- Stream AI summary output as it arrives; before the first token, show the real processing stage and elapsed time. Supported reasoning tags are shown while active and folded after completion without entering the saved summary.
- Show recording duration and staged stop/finalization progress instead of leaving long local processing unexplained.
- Use the interface in `zh-CN` or `en-US`; UI, transcription, and summary languages are configured independently.
- Recover audio checkpoints and transcript state after an interrupted recording.

### Privacy and network boundary

Mingtily has no telemetry client, usage analytics, advertising identifier, or Mingtily-hosted account service. Automatic update checks are disabled by default.

- Cold start and ordinary local use are designed not to contact non-loopback services.
- Localhost discovery, such as checking an explicitly configured Ollama endpoint, stays on the device.
- Model downloads start only after a user action, and completing a download never silently switches the active Provider or model.
- Users can manually check GitHub Releases or explicitly enable automatic checks in Settings. Update requests contain application/platform metadata required by Tauri, but no transcript or meeting content.
- External LLM requests occur only after the user configures or invokes that Provider. The relevant transcript content leaves the device in that case.
- Diagnostic logs rotate locally at five files of up to 5 MB each. They are never uploaded automatically and can be exported only from Settings after an explicit user action.
- Diagnostic exports replace the user's home-directory path and obvious credential-bearing log lines. Review an export before sharing it because filenames, device names, and error context may still be useful to diagnosis.

See [PRIVACY_POLICY.md](PRIVACY_POLICY.md) for the full boundary.

### Models and Providers

Large model weights are not bundled with the application. Models are downloaded into Mingtily's application-data directory only when requested.

| Capability | Current choices | Notes |
|---|---|---|
| Speech recognition | Whisper, Parakeet TDT 0.6B v2/v3, SenseVoice Small int8, Paraformer Small int8, Paraformer Streaming zh/en int8, Qwen3-ASR 0.6B int8, FunASR Nano int8 | SenseVoice is the recommended Chinese choice and supports forced Mandarin or Cantonese. Parakeet is English-only. Offline Paraformer is lightweight; Beta live enhancement uses a streaming model for provisional text and a separate finalized model for saved transcripts. Qwen3-ASR and FunASR Nano are larger multilingual Beta options. |
| Punctuation restoration | Sherpa ONNX CT-Transformer zh/en int8 | Optional local post-processing for final SenseVoice segments; approximately 62 MiB to download and fail-open when unavailable. |
| Speaker diarization | Sherpa ONNX `sherpa-v1` | Pyannote segmentation 3.0 int8 plus 3D-Speaker ERes2Net; approximately 47 MB to download. |
| Built-in summaries | Qwen 3.5 2B/4B, Gemma 3 1B/4B | Optional GGUF downloads; local inference uses the bundled `llama-helper` sidecar. |
| Local service summaries | Ollama | Defaults to a loopback endpoint and can use models already managed by Ollama. |
| External summaries | OpenAI, Anthropic, Groq, OpenRouter, OpenAI Compatible | Transcript content is sent directly to the configured endpoint. |

Fresh installs use SenseVoice Small int8 as the default transcription choice. Its download is approximately 158 MiB (about 226 MiB installed), and onboarding never downloads it until the user clicks Download.

Parakeet models are English-only. Parakeet v3 is pinned to revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce` of [`istupakov/parakeet-tdt-0.6b-v3-onnx`](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) and verified with built-in SHA256 values.

### Platform status

| Platform | Status | Current scope |
|---|---|---|
| macOS Apple Silicon | Primary development platform | Tagged unsigned DMG/app releases plus integrity-signed updater artifacts; microphone, system audio, Metal Whisper, import, speaker diarization, and summaries. |
| Windows x64 | CI/dev-test target | Tagged unsigned MSI/NSIS releases plus updater artifacts; audio and installer behavior still need broader hardware testing. |
| Linux x64 | CI/dev-test target | Unsigned manual AppImage/DEB builds and pull-request cold-start network checks. Tagged updater releases are paused until the Silero/ORT and static Sherpa-ONNX runtime conflict is resolved and recording is revalidated. |

Code signing, Apple notarization, and production installer signing are deferred until the project has appropriate Mingtily-owned credentials and community demand.

### Using the application

1. Open **Settings → Models** and download a local speech-recognition model.
2. Open **Settings → Services** and select the installed transcription Provider and model.
3. When using SenseVoice, optionally download punctuation restoration for more consistent Chinese and English finalized transcripts.
4. In **Settings → Services → Custom terminology**, add names and domain terms plus optional “recognized as → replace with” exact corrections. Qwen3-ASR, FunASR Nano, and Whisper also receive terms as model prompts; other models still apply exact corrections to finalized saved text.
5. Sherpa ONNX users that need compatibility rules can select one pre-generated FST in the collapsed advanced options. Mingtily does not generate FST files, download Pynini, or pass multiple rules to the native layer.
6. For continuously revised live text, download **Paraformer Streaming zh/en int8**, choose **Beta · live enhancement** in **Settings → Services**, then select the streaming and finalized models separately.
7. Optionally download and enable speaker diarization. If the expected meeting size is known, specify 1–10 speakers in **Settings → Services**.
8. Start a recording or enable Beta import/retranscription. After a meeting finishes, use the meeting-details speaker manager to merge multiple detected labels into one participant.
9. Configure Built-in AI, Ollama, or an external Provider only when summaries are needed. Background summaries continue across navigation, while the sidebar shows running and unread terminal states.
10. In **Settings → General**, manually check for releases or explicitly enable automatic checks.

Meeting-level speaker mappings do not rewrite old summaries automatically; regenerate the summary when needed. Model downloads also never silently change the active Provider or model.

### Local development

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

Local development and testing run as **Mingtily Dev** (`com.mingcheng.mingtily.dev`) by default. It has separate app data, logs, permissions, and WebView storage, so it cannot read or migrate data from an installed production copy of Mingtily.

Build an unsigned local test bundle:

```bash
cd frontend
pnpm tauri:build
```

This command also produces Mingtily Dev. Only a tagged Release workflow overlays the production configuration and produces `Mingtily` (`com.mingcheng.mingtily`).

### GitHub Actions

- `Validation` runs app-identity, frontend, i18n, network-boundary, formatting, Rust test, and Rust check gates.
- `Build and Test - Linux` also builds an unsigned Linux package on relevant pull requests and checks cold-start network connections under `strace`.
- The manual `Build and Test - DevTest`, macOS, Windows, and Linux workflows produce unsigned development artifacts without repository secrets.
- After `main` passes Validation, a maintainer creates a draft release for a new `vX.Y.Z` tag with their authenticated GitHub account. Creating that draft also creates the tag and triggers the workflow; the workflow rejects bot-authored drafts, builds the macOS Apple Silicon and Windows x64 release targets, uploads signed updater archives and `latest.json`, then publishes after both platforms succeed. Linux remains a separate unsigned development-build target for now.
- Apple notarization and operating-system installer signing remain deferred.

On Linux, install the WebKitGTK, app-indicator, ALSA, X11, and packaging dependencies shown in [`.github/workflows/build-linux.yml`](.github/workflows/build-linux.yml). See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for platform setup.

### Validation

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

### Repository layout

- `frontend/`: supported Next.js UI and Tauri application.
- `frontend/src-tauri/`: Rust audio, ASR, diarization, persistence, diagnostics, and Provider commands.
- `llama-helper/`: built-in local summary sidecar.
- `backend/`: unsupported upstream Python/FastAPI archive; do not add new Mingtily features there.
- `.github/workflows/`: secret-free validation/manual builds plus the tagged GitHub Release workflow.

See [AGENTS.md](AGENTS.md) for durable engineering boundaries and [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

### Known limitations

- On one Windows x64 test device, SenseVoice installation completed but the app exited during the post-install model-status refresh. Restarting preserved the installed model; the faulting Windows module is still under investigation.
- Parakeet is English-only and does not support Chinese. FunASR Nano and both Paraformer choices use automatic language detection. SenseVoice, Qwen3-ASR, and Whisper accept supported fixed-language hints.
- In Beta live enhancement, Paraformer Streaming shows a provisional hypothesis that can change while the user speaks. A separate finalized model transcribes completed VAD segments; provisional text is never persisted.
- Live speaker labels appear after a VAD segment finishes; they are not token-level labels.
- Overlapping speech is assigned to the dominant speaker and is not transcribed twice.
- Automatic speaker detection is heuristic. For a known meeting size, choose 1–10 speakers to cap live identities and guide final correction.
- Speaker names are meeting-local. Mingtily does not provide cross-meeting identity memory, voiceprint-based real-name identification, or name search; successful retranscription clears that meeting's mapping.
- The first custom-terminology release is global-only and changes finalized saved text. It does not provide meeting-level overrides or exact correction for provisional streaming text.
- Platform installers are currently unsigned development artifacts; only Tauri updater payloads carry Mingtily integrity signatures.

### Roadmap

See [ROADMAP.md](ROADMAP.md) for future Provider integrations, streaming performance work, offline knowledge features, and the path to 1.0 stability.

### License and attribution

Mingtily is released under the MIT License. The original Meetily copyright notice remains in [LICENSE.md](LICENSE.md).

Model weights and third-party components may use different licenses. Review [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) before redistribution.

<p align="right"><a href="#readme-zh">返回中文 ↑</a></p>
