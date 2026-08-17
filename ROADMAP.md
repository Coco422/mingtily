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
- **隐私边界可验证**：不加入使用分析或广告追踪；更新检查默认关闭，用户明确启用后才访问 GitHub Release；其他远程调用前明确说明数据去向。

## 当前基础：0.7.0

0.7.0 在现有本地优先录音与转写能力上，补齐可跨页面恢复的后台摘要任务、会议级说话人命名与合并，以及面向普通用户的全局自定义术语和精确纠错。

- Mingtily 品牌、独立 bundle identifier 和全新本地数据空间。
- 移除遥测、PRO/订阅、上游营销和 Meetily 私有更新基础设施。
- `zh-CN`、`en-US` 全应用国际化，界面语言与转写语言相互独立。
- 设置页重构为“常规 / 录音 / 模型 / 服务 / Beta”；模型资产使用按需加载和折叠分组，Services 只初始化当前转写 Provider。
- Models 统一管理 Whisper、Parakeet、Speaker Diarization、本地摘要和 Ollama 模型资产。
- Services 统一选择转写、说话人分离和 AI 摘要的 Provider 与模型。
- 本地 Whisper、仅英文的 Parakeet、SenseVoice、Offline Paraformer、Qwen3-ASR 和 FunASR Nano 转写，SenseVoice 作为推荐中文模型。
- 可选的本地中英标点恢复模型，为 SenseVoice 最终转写片段补充标点；模型缺失或失败时保留原始 ASR 文本。
- 实时录音、文件导入和重新转写统一使用 `TranscriptionProvider` 生命周期。
- Opus-in-M4A 导入，以及 Sherpa ONNX 说话人分离。
- 实时 provisional speaker label，停止录音后的全局 speaker 校正。
- 最终 speaker 校正按五分钟窗口处理长会议，避免完整 PCM 常驻内存；短音频片段在进入 ASR 前静默过滤。
- 保留 Built-in AI、Ollama 和用户主动配置的外部 LLM Provider。
- 本地模型仅在用户点击后下载；Parakeet 使用固定 revision 和 SHA256 校验。
- 隐私友好的本地滚动日志与用户主动诊断导出，不自动上传。
- PR 自动执行 i18n、前端构建、网络边界静态审计、Rust fmt/test/check。
- Linux PR 构建无签名 DEB，并对冷启动的非 loopback 连接进行运行时检查。
- macOS Apple Silicon、Windows x64 和 Linux x64 提供不依赖 secrets 的手动无签名构建。
- tag 工作流会生成 macOS Apple Silicon 与 Windows x64 GitHub Release、Tauri updater 签名产物和 `latest.json`；Linux updater 暂停到 ONNX Runtime 冲突修复并完成真实录音验证之后，Apple Developer、DigiCert 与 notarization 仍未启用。
- 录音 transcript 保存、speaker 最终标签、导入格式、模型损坏和恢复状态具有直接回归测试。

## 0.7.0：后台摘要、说话人聚合与自定义术语

- 摘要生成由应用级任务中心管理；离开会议详情后继续运行，返回时恢复 loading、流式正文、完成或失败状态。
- 侧边栏展示运行状态与未读终态；失败和应用重启中断时保留旧摘要，并提供持久错误反馈和重试入口。
- 摘要任务通过 SQLite 原子状态防止同一会议重复启动，应用启动时将遗留任务标记为 interrupted。
- 会议内可将多个检测标签合并并命名为同一人物，显示、颜色、复制和后续摘要输入统一使用解析后的姓名。
- Services 新增全局“自定义术语”，支持普通词表、大小写敏感的精确纠错以及折叠的高级 FST 兼容选项。
- 术语提示覆盖 Qwen3-ASR、FunASR Nano 与 Whisper；其他模型仍对 finalized 文本应用非级联精确纠错。
- FST 配置和 Sherpa ONNX 运行时严格限制为单一活动规则，旧多选配置必须重新选择后才能使用。

## 0.6.3：中文默认转写与可跳过初始化下载

- 全新安装默认选择 SenseVoice Small int8，不再默认使用仅支持英文的 Parakeet；SenseVoice 支持普通话、粤语、英语、日语和韩语，下载约 158 MiB、安装后约 226 MiB。
- 初始化教程中的转写与摘要模型均改为用户主动下载，并可通过“跳过，稍后下载”直接完成首次设置。
- 跳过后可随时在 Models 下载模型，并在 Services 显式选择 Provider 与模型；模型下载完成不会静默切换现有配置。
- 已保存转写配置的用户继续沿用原 Provider 和模型，不因升级被覆盖。
- 已知问题：一台 Windows x64 测试设备在 SenseVoice 安装完成后的模型状态刷新阶段发生进程退出；模型文件会保留且无需重新下载，故障模块仍在根据 Windows 事件记录排查。

## 0.6.2：最终转写增强与设置页整理

- 修复录音按钮、导航自动启动和侧边栏事件可能重复触发后端录音的问题；重复启动拒绝会与真实后端状态核对，不再把已经成功开始的录音显示为失败。
- Qwen3-ASR 与 FunASR Nano 支持 finalized 动态热词；热词可以在 Services 提前配置，只作用于保存的最终转写，不进入 Online Paraformer provisional 文本。
- 新增 FunASR Nano int8 finalized Beta 模型，保留自动语言检测、ITN、内置标点和用户显式下载、SHA256 校验边界。
- 新增可选的 Sherpa 中文同音词替换：用户主动下载 lexicon、导入预生成 `.fst` 规则并在 Services 启用；资源缺失或运行失败时保留原始 ASR 文本。
- Models 使用折叠分组并延迟挂载非当前区域；Whisper 默认只展开 Small，其余模型收进高级区域，Parakeet 默认折叠并明确仅支持英文、不支持中文。
- Services 不再无条件初始化 Whisper 与 Parakeet，只在当前 Provider 被选择时加载对应模型列表；同一进程内复用初始化任务。
- 热词入口始终可见，当前模型不支持时仍可提前保存，并明确支持范围和 finalized-only 语义。

## 0.6.1：实时反馈、说话人数与更新链路

本版本完成以下体验与可靠性改进；跨平台真实设备和操作系统签名验证仍按后续里程碑推进。

- 录音界面持续显示有效录音时长；停止后按音频停止、剩余转写、模型释放、录音保存和说话人校正展示阶段与进度。
- 修复 VAD 强制 flush 片段的时间轴换算，避免停止录音时末尾片段时间翻倍。
- SenseVoice 可选接入本地中英文标点恢复，模型缺失或运行失败时保留原始文本。
- 内置 AI、OpenAI、Anthropic、Groq、OpenRouter、Ollama 和 OpenAI Compatible 摘要支持真实流式输出；`<think>` 内容实时展示、结束后折叠，且不进入最终摘要。
- 新增可选的 Online Paraformer zh/en int8：
  - Models 仅在用户点击后下载固定 revision 与 SHA256 校验资产，下载完成不自动切换当前服务。
  - Services 提供“稳定模式 · 单模型”和“Beta · 实时增强”；Beta 模式分别选择流式模型与最终模型。
  - 流式模型接收连续混音音频，通过独立 revision 事件展示可修订文本；最终模型独立处理 VAD 完成后的长段。
  - provisional hypothesis 不写入 SQLite、IndexedDB 或 transcript JSON；只有最终模型输出负责持久化与 speaker label。
  - 流式与最终模型分别校验安装状态，不能选择同一个连续流式模型；双模型推理均运行在采集热路径之外。
- 说话人分离支持“自动 / 指定 1–10 人”：自动模式使用更保守的聚类和短句建新身份规则；指定人数会限制实时身份数量，并用于停止后的全局校正。
- 设置页提供手动检查更新和默认关闭的自动检查开关；启用后只访问 Mingtily GitHub Release，不发送会议数据。
- tag 发布工作流先创建草稿，macOS Apple Silicon 与 Windows x64 成功后上传安装包、签名 updater 产物和 `latest.json`，最后再发布 Release。

## 0.6.x：后续维护项

目标：在不扩大产品边界的前提下，继续提高 0.6.3 的真实设备可靠性和社区可用性。

- 补齐录音、导入、重新转写、speaker label、摘要和数据恢复的回归测试。
- 持续清理 i18n 遗漏、窄窗口布局和中英文文案长度问题。
- 为公开 README 补充全新的 Mingtily 界面截图，不复用旧 Meetily 素材。
- 扩大网络边界运行时测试，覆盖启动后的普通浏览与录音准备流程。
- 统一模型 manifest、断点下载、SHA256、损坏检测和原地修复行为。
- 将构建期 FFmpeg sidecar 从当前上游二进制镜像迁移到固定版本、带 SHA256 且可复现的独立来源。
- 清理 workspace member 中当前被 Cargo 忽略的 `[patch.crates-io]` 与 `[profile.release]` 配置，并用根 workspace 配置表达真实构建意图。
- 验证 Windows x64 与 Linux x64 的编译、安装和基础录音功能，明确仍不支持的能力。
- 统一 Linux 下 Silero/Parakeet 使用的 ORT 与 Sherpa-ONNX 静态运行时，恢复 VAD 原生测试、真实录音验证和 tagged updater 产物。
- 根据 CI 和真实设备结果修复 macOS、Windows、Linux 打包差异。
- 补充无签名安装包的安装、系统拦截提示和卸载说明。

验收标准：主要录音路径无数据丢失；三平台构建状态明确；无签名安装包的限制有清晰说明；仓库不依赖 Meetily 的私有服务或凭证。macOS Developer ID、notarization 和 Gatekeeper 发布验证不作为 0.6.x 门禁。

## 0.6：中文 ASR 与统一转写架构（已完成）

目标：让实时录音、文件导入和重新转写共享同一套 Provider 生命周期，并提供更适合中文的默认选择。

- 引入统一 `TranscriptionProvider`，移除 `use_parakeet` 等硬编码分支。
- 为模型声明语言控制、ITN、标点、热词、时间戳和流式能力。
- 增加 SenseVoice Small int8：
  - 支持普通话、粤语、英语、日语和韩语。
  - 支持自动语言识别和强制 `zh`。
  - 默认启用中文 ITN；可选下载独立的中英标点恢复模型。
- SenseVoice 继续复用现有 Silero VAD、speaker diarization 和分段后 ASR 流程。
- 增加 Offline Paraformer Small int8 作为轻量中英模型；沿用 VAD 分段后转写，语言自动判断。
- 增加 Qwen3-ASR 0.6B int8 作为高质量多语言 Beta 档；由于下载、内存和算力需求明显更高，不设为默认模型。
- Whisper 与 Parakeet 继续保留，不强制迁移用户模型。
- 所有 ASR 模型复用统一 manifest、`.part`、SHA256、staging 和原子安装机制。
- Models 只管理资产，下载完成不会自动切换当前 Provider；Services 负责选择实际使用的模型。

验收标准：用户的中文 Opus-in-M4A 文件在强制 `zh` 时稳定输出中文；三条转写路径使用同一 Provider 接口；现有 speaker、时间轴和持久化行为不回归。

## 0.7：实时中文与性能分档（进行中）

目标：从“VAD 段完成后出现文本”推进到真正连续的中文流式转写。

- 已完成 Online Paraformer bilingual zh/en 的首版连续流式转写、partial hypothesis 和 final revision。模型为 `csukuangfj/sherpa-onnx-streaming-paraformer-bilingual-zh-en`，固定 revision `8e40c43232a1c5c66c82111efc5820d3accca11b`；int8 资产约 226 MiB。
- 已完成独立 session 生命周期与 provisional/final 事件契约，不把临时假设伪装成普通持久化 transcript。
- 已直接使用 sherpa-onnx `OnlineRecognizer` / `OnlineStream` 的 `is_ready`、`is_endpoint`、`reset` 与 `RecognizerResult.is_final`，推理与音频采集、混音热路径隔离。
- 已完成 Beta 双线识别策略：Online Paraformer 负责连续临时文本，用户另选 SenseVoice、Offline Paraformer、Qwen3-ASR、Whisper 或 Parakeet 作为最终模型；下载模型不会自动启用该策略。
- 已在 macOS Apple Silicon 无签名安装包中完成模型下载、显式服务切换、真实系统音频流式修订、录音时长、停止进度和 final-only 持久化 smoke test。
- 待补充双模型峰值内存、流式队列背压、长会议 soak test、不同设备的录音启动耗时、首字延迟与 revision 稳定性数据。
- Offline Paraformer Large int8：本地质量优先档。
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
- 不启用默认后台更新；更新检查必须由用户手动触发或明确开启，安装和重启仍由用户确认。
- 不强制所有 AI 推理都使用本地模型。
- 不在近期重写 JobManager、Cancel、音频混音或现有 VAD 算法。
- 不做说话人实名识别、声纹注册或默认的跨会议身份记忆。
- 不把大型模型直接打包进安装程序。

## 如何参与

当前最有价值的贡献是可复现的问题、真实语言样本的匿名化测试结果、跨平台构建修复、i18n 改进和模型性能数据。实现新 Provider 或新模型前，优先确认它能接入现有能力接口和统一模型管理机制。
