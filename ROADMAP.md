# Mingtily Roadmap

Mingtily 的目标是成为一款面向个人、开发者和小团队的本地优先 AI 会议工具：录音、音频、转写、说话人标签和本地模型默认留在设备上，同时允许用户主动选择外部 AI 服务获得更好的总结质量与性能。

> Mingtily is an independent community fork of Meetily. It is not affiliated with or endorsed by the Meetily project.

本路线图描述方向和验收边界，不承诺固定发布日期。里程碑会根据模型质量、跨平台构建结果和真实使用反馈调整。

## 产品原则

- **本地优先，不等于本地限定**：核心会议数据默认保存在本机，外部 Provider 由用户明确配置和调用。
- **启动默认离线**：冷启动和普通浏览不应产生非用户触发的外网请求；localhost 服务探测除外。
- **模型与服务分离**：Models 管理本地资产，Services 决定各项能力实际使用的 Provider 和模型。
- **中文体验优先**：优先改善中文、粤语及中英混合场景，同时保持英文和现有模型可用。
- **失败可降级**：说话人分离、摘要或单个 Provider 失败时，不应丢失录音和原始转写。
- **隐私边界可验证**：不加入使用分析、广告追踪或后台更新；远程调用前明确说明数据去向。

## 当前基础：0.5.2

0.5.2 完成了 Mingtily 独立 fork 的第一轮稳定化，重点是建立清晰的产品身份、离线边界、可扩展配置结构和可持续验证基础。

- Mingtily 品牌、独立 bundle identifier 和全新本地数据空间。
- 移除遥测、后台更新、PRO/订阅及上游营销入口。
- `zh-CN`、`en-US` 全应用国际化，界面语言与转写语言相互独立。
- 设置页重构为“常规 / 录音 / 模型 / 服务 / Beta”。
- Models 统一管理 Whisper、Parakeet、Speaker Diarization、本地摘要和 Ollama 模型资产。
- Services 统一选择转写、说话人分离和 AI 摘要的 Provider 与模型。
- 本地 Whisper、Parakeet 转写，Opus-in-M4A 导入，以及 Sherpa ONNX 说话人分离。
- 实时 provisional speaker label，停止录音后的全局 speaker 校正。
- 保留 Built-in AI、Ollama 和用户主动配置的外部 LLM Provider。
- 本地模型仅在用户点击后下载；Parakeet 使用固定 revision 和 SHA256 校验。
- 隐私友好的本地滚动日志与用户主动诊断导出，不自动上传。
- PR 自动执行 i18n、前端构建、网络边界静态审计、Rust fmt/test/check。
- Linux PR 构建无签名 DEB，并对冷启动的非 loopback 连接进行运行时检查。
- macOS Apple Silicon、Windows x64 和 Linux x64 提供不依赖 secrets 的手动无签名构建。
- 当前工作流不包含 Apple Developer、DigiCert、notarization 或正式 Release 自动化。
- 录音 transcript 保存、speaker 最终标签、导入格式、模型损坏和恢复状态具有直接回归测试。

## 0.5.x：后续维护项

目标：在不扩大产品边界的前提下，继续提高 0.5.2 的真实设备可靠性和社区可用性。

- 补齐录音、导入、重新转写、speaker label、摘要和数据恢复的回归测试。
- 持续清理 i18n 遗漏、窄窗口布局和中英文文案长度问题。
- 为公开 README 补充全新的 Mingtily 界面截图，不复用旧 Meetily 素材。
- 扩大网络边界运行时测试，覆盖启动后的普通浏览与录音准备流程。
- 统一模型 manifest、断点下载、SHA256、损坏检测和原地修复行为。
- 将构建期 FFmpeg sidecar 从当前上游二进制镜像迁移到固定版本、带 SHA256 且可复现的独立来源。
- 清理 workspace member 中当前被 Cargo 忽略的 `[patch.crates-io]` 与 `[profile.release]` 配置，并用根 workspace 配置表达真实构建意图。
- 验证 Windows x64 与 Linux x64 的编译、安装和基础录音功能，明确仍不支持的能力。
- 根据 CI 和真实设备结果修复 macOS、Windows、Linux 打包差异。
- 补充无签名安装包的安装、系统拦截提示和卸载说明。

验收标准：主要录音路径无数据丢失；三平台构建状态明确；无签名安装包的限制有清晰说明；仓库不依赖 Meetily 的私有服务或凭证。macOS Developer ID、notarization 和 Gatekeeper 发布验证不作为 0.5.x 门禁。

## 0.6：中文 ASR 与统一转写架构

目标：让实时录音、文件导入和重新转写共享同一套 Provider 生命周期，并提供更适合中文的默认选择。

- 引入统一 `TranscriptionProvider`，移除 `use_parakeet` 等硬编码分支。
- 为模型声明语言控制、ITN、标点、热词、时间戳和流式能力。
- 增加 SenseVoice Small int8：
  - 支持普通话、粤语、英语、日语和韩语。
  - 支持自动语言识别和强制 `zh`。
  - 默认启用中文 ITN 与标点。
- SenseVoice 继续复用现有 Silero VAD、speaker diarization 和分段后 ASR 流程。
- Whisper 与 Parakeet 继续保留，不强制迁移用户模型。
- 所有 ASR 模型复用统一 manifest、`.part`、SHA256、staging 和原子安装机制。

验收标准：用户的中文 Opus-in-M4A 文件在强制 `zh` 时稳定输出中文；三条转写路径使用同一 Provider 接口；现有 speaker、时间轴和持久化行为不回归。

## 0.7：实时中文与性能分档

目标：从“VAD 段完成后出现文本”推进到真正连续的中文流式转写。

- Online Paraformer：连续流式转写和 partial hypothesis。
- Offline Paraformer Small int8：轻量低资源档。
- Offline Paraformer Large int8：本地质量优先档。
- FunASR Nano：作为实验模型评估质量、内存、包体和许可证，不作为默认下载。
- 为不同模型记录首段延迟、实时率、峰值内存和长会议稳定性。
- 保持 speaker label 与流式文本的时序一致，不承诺 token 级说话人标签。

验收标准：普通 Apple Silicon 设备可持续实时中文转写；用户能够根据设备能力选择轻量或质量档；录音采集热路径不被模型推理阻塞。

## 0.8：本地与国内 AI Provider 生态

目标：通过少量稳定协议覆盖本地服务、国际 Provider 和国内 OpenAI Compatible 服务。

- 建立 Provider Registry，核心协议收敛为：
  - Ollama
  - OpenAI Compatible
  - Anthropic
  - Gemini
  - Built-in Local AI
- 增加 LM Studio、vLLM、Xinference、SiliconFlow、火山方舟、DashScope 和 DeepSeek preset。
- 自动检测仅扫描明确的 localhost 常用端口，不扫描局域网。
- 支持一键检测 Ollama、LM Studio 和已启动的本地兼容服务。
- Provider 的模型发现、连接测试、能力声明和错误展示使用统一接口。
- 外部 Provider 调用前持续展示“转写内容会发送到所配置服务”的边界说明。

验收标准：新增 OpenAI Compatible 服务不需要复制一套请求实现；未配置外部 Provider 时应用仍可完整使用录音和本地转写。

## 0.9：完整离线 AI 会议能力

目标：在稳定的录音与转写数据层上扩展会议知识处理能力。

- 一键离线模式：阻断所有非 loopback 请求，并关闭外部自动摘要。
- 支持离线模型包导入和导出，不依赖应用内下载。
- MCP 工具与上下文接入。
- 面向单场会议和会议集合的本地 RAG。
- 企业知识库辅助总结，保留引用来源。
- 电话录音、访谈、销售、项目复盘等结构化模板。
- 会议数据可移植导出，避免形成应用锁定。

验收标准：断网环境可完成录音、转写、说话人分离、浏览和本地摘要；知识库回答可定位来源；离线模式不存在隐式远程调用。

## 1.0：稳定版

目标：形成可长期维护、可验证发布、适合真实会议使用的桌面产品。

- macOS Apple Silicon、Windows x64 和 Linux x64 的支持矩阵与已知限制清晰可查。
- 可靠处理长会议、崩溃恢复、磁盘空间不足和模型损坏等错误路径。
- 数据格式、配置兼容和升级策略稳定。
- 完成可访问性、键盘操作和主要界面的响应式检查。
- 安全、隐私、第三方许可证和模型来源审计完成。
- 可复现的 CI 构建、签名、notarization、校验和发布说明流程。
- 当具备 Mingtily 自有 Apple/Windows 凭证且社区确有安装需求时，再启用正式签名与 macOS notarization。

## 持续演进方向

这些方向可以跨里程碑推进，但不会阻塞核心录音和转写体验。

- **Speaker Diarization**：从当前 Pyannote segmentation + ERes2Net 扩展到 ERes2NetV2、CAM++ 和更稳健的聚类策略。
- **说话人体验**：在数据契约稳定后评估手动重命名、合并和纠错；默认不做跨会议声纹记忆。
- **诊断能力**：本地滚动日志、隐私脱敏、诊断包导出和性能指标展示。
- **数据互操作**：Markdown、JSON、字幕和常用会议格式导出。
- **社区模型**：通过 manifest 增加模型，而不是为每个模型复制下载器和设置页面。
- **RAG 功能**：让会议内容成为可检索、可追溯的知识来源，同时保持录音与转写是核心体验。

## 当前不做

- 不加入遥测、行为分析、广告追踪或强制账户系统。
- 不恢复后台自动更新；发布包由用户主动下载和安装。
- 不强制所有 AI 推理都使用本地模型。
- 不在近期重写 JobManager、Cancel、音频混音或现有 VAD 算法。
- 不做说话人实名识别、声纹注册或默认的跨会议身份记忆。
- 不把大型模型直接打包进安装程序。

## 如何参与

当前最有价值的贡献是可复现的问题、真实语言样本的匿名化测试结果、跨平台构建修复、i18n 改进和模型性能数据。实现新 Provider 或新模型前，优先确认它能接入现有能力接口和统一模型管理机制。
