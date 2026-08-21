import { settingsExtraResources } from './settingsExtraResources';

export const settingsResources = {
  'en-US': {
    ...settingsExtraResources['en-US'].settings,
    title: 'Settings', back: 'Back',
    tabs: { general: 'General', recordings: 'Recordings', models: 'Models', services: 'Services', beta: 'Beta' },
    actions: { save: 'Save', saving: 'Saving…' },
    general: { loading: 'Loading preferences…', appLanguage: 'App language', appLanguageDescription: 'Changes the interface immediately. Transcription language is configured separately.', notifications: 'Notifications', notificationsDescription: 'Notify me when a meeting recording starts or ends.', storage: 'Data storage', storageDescription: 'View where Mingtily keeps local data.', recordingsFolder: 'Meeting recordings', openFolder: 'Open folder', noteLabel: 'Note:', storageNote: 'The database and models share Mingtily’s app-data directory.', diagnostics: 'Diagnostic logs', diagnosticsDescription: 'Mingtily keeps up to five local 5 MB log files to help investigate recording, model, and startup problems.', diagnosticsPrivacy: 'Logs are never uploaded automatically. Exporting is optional and replaces your home-directory path and obvious credential-bearing lines.', exportDiagnostics: 'Export diagnostics', exportingDiagnostics: 'Exporting…', diagnosticsExported: 'Diagnostics exported', diagnosticsExportedDescription: '{{count}} log file(s) included.', diagnosticsExportFailed: 'Could not export diagnostics', updates: { title: 'App updates', description: 'Automatically check GitHub Releases after startup once you enable this option.', privacy: 'Disabled by default. Enabling it contacts github.com; no transcript or meeting data is sent.', idle: 'Automatic checks are optional.', developmentDisabled: 'Updates are disabled in Mingtily Dev.', checking: 'Checking for updates…', checkNow: 'Check now', current: 'Mingtily is up to date.', available: 'Mingtily {{version}} is available.', download: 'Download update', downloading: 'Downloading Mingtily {{version}}…', downloadingProgress: 'Downloading Mingtily {{version}}… {{progress}}%', ready: 'Mingtily {{version}} is installed and ready to restart.', restartNow: 'Restart now', restartBlocked: 'Stop the current recording before restarting to update.', restartFailed: 'Could not restart Mingtily.', checkFailed: 'Could not check for updates.', installFailed: 'Could not install the update.' } },
    recordings: { title: 'Recording settings', description: 'Choose how meeting audio is saved and which devices are used by default.', saveAudio: 'Save audio recordings', saveAudioDescription: 'Keep the audio file after recording stops.', saveLocation: 'Save location', defaultFolder: 'Default folder', fileFormat: 'File format:', filenamePattern: 'Files use the name recording_YYYYMMDD_HHMMSS.{{extension}}', disabledHint: 'Audio saving is off. Turn on “Save audio recordings” to keep meeting audio.', startNotification: 'Recording reminder', startNotificationDescription: 'Remind you to notify participants when recording begins.', defaultDevices: 'Default audio devices', defaultDevicesDescription: 'Mingtily will preselect these microphone and system-audio devices for new recordings.', preferenceSaved: 'Preference saved', preferenceSaveFailed: 'Could not save preference', defaultDevice: 'Default', devicesSaved: 'Audio devices saved', devicesSavedDescription: 'Microphone: {{mic}} · System audio: {{system}}', devicesSaveFailed: 'Could not save audio devices' },
    beta: { title: 'Beta features', description: 'These features are still being tested and may have rough edges.', features: { importAndRetranscribe: { name: 'Import & retranscribe', description: 'Import audio as a new meeting or run transcription again with different settings.' }, customTranscriptionPipelines: { name: 'Custom transcription pipelines', description: 'Choose separate live, finalized, and post-meeting processing paths.' }, experimentalAsrModels: { name: 'Experimental ASR models', description: 'Show experimental streaming and high-resource speech recognition models.' } }, note: 'Turning a feature off hides its entry points and disables it at runtime. Downloaded models and saved choices are kept.' },
    pipelineDecisions: { customPipelineFallback: 'Custom pipelines are gated off; the Balanced profile is active without changing your saved choices.', stableFallback: 'The experimental model is gated off; SenseVoice will be used without changing your saved selection.', punctuationDisabled: 'External punctuation is disabled because the model already provides punctuation or the enhancement is off.' },
    pipelineJobQueued: 'Reprocessing was queued and will continue in the background.',
    pipeline: { title: 'Transcription pipeline', description: 'Control live transcription, saved results, and resource-aware post-meeting work.', preset: 'Profile', presets: { fast: 'Fast', balanced: 'Balanced', quality: 'High quality', custom: 'Custom' }, liveMode: 'Live path', liveModes: { off: 'Off', 'vad-segmented': 'VAD segmented', 'continuous-preview': 'Continuous preview' }, streamingProvider: 'Streaming Provider', streamingModel: 'Streaming model', resourceMode: 'Resource mode', resources: { eco: 'Eco', balanced: 'Balanced', fast: 'High performance' }, postMeetingAsr: 'Post-meeting ASR', speakerRefinement: 'Speaker refinement', punctuation: 'Automatic punctuation', terminology: 'Terminology correction', runOnBattery: 'Run automatic jobs on battery', policies: { off: 'Off', manual: 'Manual', auto: 'Automatic' }, speakerPolicies: { off: 'Off', manual: 'Manual', 'background-auto': 'Automatic in background' }, postProvider: 'Post-meeting Provider', postModel: 'Post-meeting model', memoryLimit: 'Memory limit (MiB)', resolvedFinalized: 'Saved transcript: {{provider}} / {{model}}', resolvedPost: 'Post-meeting: {{provider}} / {{model}}', estimate: 'Estimated memory: {{memory}} MiB · {{workers}} worker(s) · {{threads}} thread(s) each', saved: 'Pipeline saved', saveFailed: 'Could not save pipeline', jobs: { asr_recompute: 'Recomputing transcript', speaker_refinement: 'Refining speakers', failedToast: 'Background meeting processing failed', pause: 'Pause', resume: 'Resume', cancel: 'Cancel', status: { pending: 'Waiting', processing: 'Processing', paused: 'Paused', failed: 'Failed' } } },
    services: {
      provider: 'Provider', model: 'Model', selectProvider: 'Select provider', selectModel: 'Select model…', selectInstalledModel: 'Select an installed model', noInstalledModel: 'No installed model is available for this provider.', searchModels: 'Search models…', loadingModels: 'Loading models…', noModels: 'No models found.', fetchingModels: 'Fetching…', fetchModels: 'Fetch models',
      transcription: { title: 'Speech recognition', description: 'Choose the local ASR provider, installed model, and default audio language.', mode: 'Recognition mode', modes: { stable: 'Stable · single model', betaLive: 'Beta · live enhancement' }, stableModeDescription: 'One model transcribes finalized VAD segments. This uses less memory and compute.', betaModeDescription: 'A streaming model shows immediate provisional text while a separate finalized model produces the saved transcript.', streamingPath: 'Live provisional path', streamingProvider: 'Streaming provider', streamingModel: 'Streaming model', streamingModelMissing: 'No continuous streaming model is installed. Download one in Models.', finalizedPath: 'Saved finalized path', finalizedProvider: 'Finalized provider', finalizedModel: 'Finalized model', saved: 'Speech recognition service saved', saveFailed: 'Could not save speech recognition service', language: 'Transcription language', autoDetect: 'Auto-detect (original language)', autoTranslate: 'Auto-detect and translate to English', languageSaved: 'Transcription language saved', languageSavedDescription: 'New recordings will use {{language}}.', languageSaveFailed: 'Could not save transcription language', parakeetLanguageTitle: 'Parakeet language support', parakeetLanguageDescription: 'Parakeet is English-only and does not support Chinese. Choose Whisper or a Sherpa ONNX model for Chinese and multilingual audio.', automaticLanguageTitle: 'Automatic language detection', automaticLanguageDescription: '{{model}} detects the language automatically and does not accept a fixed language.', currentLanguage: 'Current:', autoDetectWarning: 'Automatic detection can choose the wrong language', autoDetectHint: 'For predictable results, choose the language spoken in the recording.', translationActive: 'English translation is on', translationDescription: 'Speech in any supported language will be translated into English.', optimizedFor: 'Transcription will be optimized for', streamingNotice: 'Provisional streaming hypotheses are display-only and may change. Only finalized VAD segments are saved to the meeting.', terminology: { title: 'Finalized terminology enhancement', description: 'Optional local enhancements applied only to saved finalized segments. Hotwords can be prepared here before switching to a supported model.', hotwords: 'Dynamic hotwords', hotwordsPlaceholder: 'One term per line, for example:\nMingtily\nSenseVoice\nQwen3-ASR', hotwordsHint: 'Qwen3-ASR and Fun-ASR Nano receive these terms at recognizer initialization. Separate terms with a new line or comma.', hotwordsUnsupported: 'The selected finalized model does not support dynamic hotwords. The terms are still saved and will apply after switching to Qwen3-ASR or Fun-ASR Nano.', homophone: 'Chinese homophone replacement', homophoneHint: 'Uses Sherpa’s local lexicon and selected pre-generated .fst rules after recognition. It only changes Chinese characters.', homophoneSherpaOnly: 'Chinese homophone replacement is available when the finalized Provider is Sherpa ONNX. Hotwords above can be configured in advance for Qwen3-ASR or Fun-ASR Nano.', lexiconMissing: 'Download the Chinese homophone lexicon in Models before enabling this option.', rulesMissing: 'Import at least one pre-generated .fst rule in Models before enabling this option.', selectRules: 'Rules used for finalized text' } },
      speaker: { title: 'Speaker diarization', description: 'Turn speaker labels on or off and choose the diarization backend.', saved: 'Speaker diarization service saved', saveFailed: 'Could not save speaker diarization service', modelMissing: 'The speaker model is not installed.', disabledHint: 'Speaker labels, imported-audio diarization, and final label refinement will be skipped. Speech recognition still works.', speakerCount: 'Speaker count', autoDetect: 'Auto detect', fixedCount: '{{count}} speakers', autoDetectHint: 'Uses conservative clustering to avoid splitting one person into several identities.', fixedCountHint: 'Caps live identities and uses this count during final speaker correction.' },
      summary: { title: 'AI summaries', description: 'Choose when summaries run and which local or external provider generates them.', remoteNotice: 'Meeting transcripts will be sent to the external provider you configure.', loadFailed: 'Could not load summary settings', saved: 'Summary service saved', saveFailed: 'Could not save summary service', autoSummary: 'Automatic summary', autoSummaryDescription: 'Generate a summary when a meeting finishes.', configuration: 'Summary provider', configurationDescription: 'Configure the AI service used to generate meeting summaries.', language: 'Summary language', languageDescription: 'Pin a default for new meetings. Auto follows the dominant transcript language.', pinLanguage: 'Set {{language}} as default', unpinLanguage: 'Remove {{language}} as default', setDefault: 'Set as default', unsetDefault: 'Unset default', removeLanguage: 'Remove {{language}}', addLanguage: 'Add language', defaultLanguageHint: 'Default: {{language}}. Click it again to unset. Up to 5 quick choices.', noDefaultLanguageHint: 'Click a language to make it the default. Up to 5 quick choices.', modelSettings: 'Provider settings', summaryModel: 'Summary model', providers: { builtin: 'Built-in AI (offline)', customOpenAI: 'OpenAI-compatible service' }, endpointRequired: 'Endpoint URL *', endpointDescription: 'Base URL for the OpenAI-compatible API.', modelNameRequired: 'Model name *', modelNameDescription: 'Model identifier sent with each request.', apiKey: 'API key', apiKeyOptional: 'API key (optional)', apiKeyPlaceholder: 'Enter API key', apiKeyPlaceholderOptional: 'Leave blank if the service does not require one', advancedOptions: 'Advanced options', testingConnection: 'Testing…', testConnection: 'Test connection', unlockApiKey: 'Unlock to edit', lockApiKey: 'Lock field', customEndpoint: 'Custom endpoint (optional)', customEndpointDescription: 'Leave blank for the default endpoint, or enter another Ollama URL.', endpointChanged: 'Fetch models from the new endpoint before saving.' }
    }
  },
  'zh-CN': {
    ...settingsExtraResources['zh-CN'].settings,
    title: '设置', back: '返回',
    tabs: { general: '常规', recordings: '录音', models: '模型', services: '服务', beta: 'Beta' },
    actions: { save: '保存', saving: '保存中…' },
    general: { loading: '正在加载设置…', appLanguage: '应用语言', appLanguageDescription: '保存后界面立即切换，无需重启。转写语言请在“服务”中单独设置。', notifications: '通知', notificationsDescription: '在会议录音开始或结束时通知我。', storage: '数据存储', storageDescription: '查看 Mingtily 本地数据的保存位置。', recordingsFolder: '会议录音', openFolder: '打开文件夹', noteLabel: '说明：', storageNote: '数据库和模型统一保存在 Mingtily 的应用数据目录。', diagnostics: '诊断日志', diagnosticsDescription: 'Mingtily 在本机最多保留 5 个日志文件，每个 5 MB，用于排查录音、模型和启动问题。', diagnosticsPrivacy: '日志不会自动上传。仅在你主动导出时生成文件，并替换用户目录路径及明显包含凭证的日志行。', exportDiagnostics: '导出诊断日志', exportingDiagnostics: '正在导出…', diagnosticsExported: '诊断日志已导出', diagnosticsExportedDescription: '已包含 {{count}} 个日志文件。', diagnosticsExportFailed: '诊断日志导出失败', updates: { title: '应用更新', description: '启用后，Mingtily 会在启动后自动检查 GitHub Release。', privacy: '默认关闭。启用后会连接 github.com，不会发送会议或转写内容。', idle: '自动检查是可选功能。', developmentDisabled: 'Mingtily Dev 已禁用应用更新。', checking: '正在检查更新…', checkNow: '立即检查', current: '当前已是最新版本。', available: '发现 Mingtily {{version}}。', download: '下载更新', downloading: '正在下载 Mingtily {{version}}…', downloadingProgress: '正在下载 Mingtily {{version}}… {{progress}}%', ready: 'Mingtily {{version}} 已安装，重启后生效。', restartNow: '立即重启', restartBlocked: '请先停止当前录音，再重启完成更新。', restartFailed: 'Mingtily 重启失败。', checkFailed: '检查更新失败。', installFailed: '更新安装失败。' } },
    recordings: { title: '录音设置', description: '设置会议音频的保存方式和默认录音设备。', saveAudio: '保存录音文件', saveAudioDescription: '录音结束后保留完整音频。', saveLocation: '保存位置', defaultFolder: '默认文件夹', fileFormat: '文件格式：', filenamePattern: '文件名格式：recording_YYYYMMDD_HHMMSS.{{extension}}', disabledHint: '当前不会保存音频。开启“保存录音文件”后，会议音频才会保留。', startNotification: '录音提醒', startNotificationDescription: '录音开始时，提醒你告知参会者。', defaultDevices: '默认音频设备', defaultDevicesDescription: '开始新录音时，Mingtily 会优先选择这些麦克风和系统音频设备。', preferenceSaved: '设置已保存', preferenceSaveFailed: '设置保存失败', defaultDevice: '默认设备', devicesSaved: '音频设备已保存', devicesSavedDescription: '麦克风：{{mic}} · 系统音频：{{system}}', devicesSaveFailed: '音频设备保存失败' },
    beta: { title: 'Beta 功能', description: '这些功能仍在测试，使用中可能遇到不完善之处。', features: { importAndRetranscribe: { name: '导入与重新转写', description: '导入音频创建会议，或使用不同设置重新生成已有会议的转写。' }, customTranscriptionPipelines: { name: '自定义转写 Pipeline', description: '分别配置实时、最终保存和会后处理路径。' }, experimentalAsrModels: { name: '实验性 ASR 模型', description: '显示实验性的流式模型和高资源占用语音识别模型。' } }, note: '关闭后会隐藏相关入口并在运行时停用；已下载模型和原配置仍会保留。' },
    pipelineDecisions: { customPipelineFallback: '自定义 Pipeline 开关已关闭，当前改用“均衡”方案，原配置仍会保留。', stableFallback: '实验模型开关已关闭，当前改用 SenseVoice，原选择仍会保留。', punctuationDisabled: '当前模型已自带标点或增强已关闭，因此不会启动外部标点模型。' },
    pipelineJobQueued: '重新处理任务已加入队列，将在后台继续执行。',
    pipeline: { title: '转写 Pipeline', description: '统一控制实时转写、最终保存以及受资源约束的会后处理。', preset: '方案', presets: { fast: '快速', balanced: '均衡', quality: '高质量', custom: '自定义' }, liveMode: '实时路径', liveModes: { off: '关闭', 'vad-segmented': 'VAD 分段', 'continuous-preview': '连续临时字幕' }, streamingProvider: '流式 Provider', streamingModel: '流式模型', resourceMode: '资源模式', resources: { eco: '省电', balanced: '均衡', fast: '高性能' }, postMeetingAsr: '会后 ASR 重算', speakerRefinement: '说话人校正', punctuation: '自动标点', terminology: '术语校正', runOnBattery: '使用电池时运行自动任务', policies: { off: '关闭', manual: '手动', auto: '自动' }, speakerPolicies: { off: '关闭', manual: '手动', 'background-auto': '后台自动' }, postProvider: '会后 Provider', postModel: '会后模型', memoryLimit: '内存上限（MiB）', resolvedFinalized: '最终转写：{{provider}} / {{model}}', resolvedPost: '会后处理：{{provider}} / {{model}}', estimate: '预计内存：{{memory}} MiB · {{workers}} 个 worker · 每个 {{threads}} 线程', saved: 'Pipeline 已保存', saveFailed: 'Pipeline 保存失败', jobs: { asr_recompute: '正在重新转写', speaker_refinement: '正在校正说话人', failedToast: '会议后台处理失败', pause: '暂停', resume: '继续', cancel: '取消', status: { pending: '等待中', processing: '处理中', paused: '已暂停', failed: '失败' } } },
    services: {
      provider: 'Provider', model: '模型', selectProvider: '选择 Provider', selectModel: '选择模型…', selectInstalledModel: '选择已安装模型', noInstalledModel: '该 Provider 暂无可用的已安装模型。', searchModels: '搜索模型…', loadingModels: '正在加载模型…', noModels: '没有找到模型。', fetchingModels: '正在获取…', fetchModels: '获取模型',
      transcription: { title: '语音转写', description: '选择本地 ASR Provider、已安装模型和默认音频语言。', mode: '识别模式', modes: { stable: '稳定模式 · 单模型', betaLive: 'Beta · 实时增强' }, stableModeDescription: '使用一个模型处理 VAD 完成后的最终片段，占用的内存和算力更少。', betaModeDescription: '流式模型即时展示临时文本，另一套最终模型生成并保存正式转写。', streamingPath: '实时临时识别', streamingProvider: '流式 Provider', streamingModel: '流式模型', streamingModelMissing: '尚未安装支持连续流式识别的模型，请前往“模型”下载。', finalizedPath: '正式转写', finalizedProvider: '最终 Provider', finalizedModel: '最终模型', saved: '语音转写服务已保存', saveFailed: '语音转写服务保存失败', language: '转写语言', autoDetect: '自动检测（保留原语言）', autoTranslate: '自动检测并翻译为英文', languageSaved: '转写语言已保存', languageSavedDescription: '新录音将使用：{{language}}。', languageSaveFailed: '转写语言保存失败', parakeetLanguageTitle: 'Parakeet 语言支持', parakeetLanguageDescription: 'Parakeet 仅支持英文，不支持中文。中文或多语言录音请使用 Whisper 或 Sherpa ONNX 模型。', automaticLanguageTitle: '自动判断语言', automaticLanguageDescription: '{{model}} 会自动判断语言，暂不接受固定语言设置。', currentLanguage: '当前：', autoDetectWarning: '自动检测可能选错语言', autoDetectHint: '如需稳定结果，建议直接选择录音中使用的语言。', translationActive: '英文翻译已开启', translationDescription: '支持的语音内容会统一翻译为英文。', optimizedFor: '转写将针对以下语言优化：', streamingNotice: '流式临时结果只用于展示，内容可能随识别修订；会议中只保存最终模型处理完成的 VAD 片段。', terminology: { title: '最终转写术语增强', description: '这些可选的本地增强只作用于保存的最终片段。可以先在这里配置热词，再切换到支持的模型。', hotwords: '动态热词', hotwordsPlaceholder: '每行一个术语，例如：\nMingtily\nSenseVoice\nQwen3-ASR', hotwordsHint: 'Qwen3-ASR 和 Fun-ASR Nano 会在初始化识别器时接收这些术语；可使用换行或逗号分隔。', hotwordsUnsupported: '当前最终模型不支持动态热词，但术语仍会保存；切换到 Qwen3-ASR 或 Fun-ASR Nano 后生效。', homophone: '中文同音词替换', homophoneHint: '识别完成后使用 Sherpa 本地词典和选中的预生成 .fst 规则，只处理中文汉字。', homophoneSherpaOnly: '中文同音词替换仅在最终 Provider 为 Sherpa ONNX 时可用。上面的热词可以提前配置，切换到 Qwen3-ASR 或 Fun-ASR Nano 后即可使用。', lexiconMissing: '请先在“模型”中下载中文同音词词典，再开启此功能。', rulesMissing: '请先在“模型”中导入至少一个预生成 .fst 规则，再开启此功能。', selectRules: '用于最终文本的规则' } },
      speaker: { title: '说话人分离', description: '开启或关闭说话人标签，并选择分离后端。', saved: '说话人分离服务已保存', saveFailed: '说话人分离服务保存失败', modelMissing: '尚未安装说话人分离模型。', disabledHint: '将跳过实时标签、导入音频分离和停止后的标签校正；语音转写仍可正常运行。', speakerCount: '说话人数', autoDetect: '自动检测', fixedCount: '{{count}} 人', autoDetectHint: '使用更保守的聚类，减少同一个人被拆成多个身份。', fixedCountHint: '实时标签最多使用该人数，停止后也按该人数进行全局校正。' },
      summary: { title: 'AI 总结', description: '设置自动总结，并选择本地或外部 AI Provider。', remoteNotice: '会议转写内容会发送到你配置的外部 Provider。', loadFailed: 'AI 总结设置加载失败', saved: 'AI 总结服务已保存', saveFailed: 'AI 总结服务保存失败', autoSummary: '自动总结', autoSummaryDescription: '会议结束后自动生成总结。', configuration: '总结 Provider', configurationDescription: '配置用于生成会议总结的 AI 服务。', language: '总结语言', languageDescription: '可固定新会议的默认语言；“自动”会跟随转写中的主要语言。', pinLanguage: '将{{language}}设为默认语言', unpinLanguage: '取消{{language}}的默认设置', setDefault: '设为默认', unsetDefault: '取消默认', removeLanguage: '移除{{language}}', addLanguage: '添加语言', defaultLanguageHint: '默认：{{language}}。再次点击可取消；最多保留 5 个快捷选项。', noDefaultLanguageHint: '点击语言即可设为默认；最多保留 5 个快捷选项。', modelSettings: 'Provider 设置', summaryModel: '总结模型', providers: { builtin: '内置 AI（离线）', customOpenAI: 'OpenAI 兼容服务' }, endpointRequired: 'Endpoint URL *', endpointDescription: 'OpenAI 兼容 API 的基础地址。', modelNameRequired: '模型名称 *', modelNameDescription: '请求中使用的模型标识。', apiKey: 'API Key', apiKeyOptional: 'API Key（可选）', apiKeyPlaceholder: '输入 API Key', apiKeyPlaceholderOptional: '服务不需要时可留空', advancedOptions: '高级选项', testingConnection: '正在测试…', testConnection: '测试连接', unlockApiKey: '解锁编辑', lockApiKey: '锁定输入框', customEndpoint: '自定义 Endpoint（可选）', customEndpointDescription: '留空使用默认地址，也可以填写其他 Ollama 地址。', endpointChanged: '请先从新 Endpoint 获取模型，再保存设置。' }
    }
  }
} as const;

Object.assign(settingsResources['en-US'].general, {
  summaryNotifications: 'AI summary outcomes',
  summaryNotificationsDescription: 'Show a system notification when a background summary finishes or fails. Meeting content is never included.',
  notificationsSaveFailed: 'Could not save notification settings',
});
Object.assign(settingsResources['zh-CN'].general, {
  summaryNotifications: 'AI 摘要结果',
  summaryNotificationsDescription: '后台摘要完成或失败时发送系统通知，通知中不会包含会议正文。',
  notificationsSaveFailed: '无法保存通知设置',
});
Object.assign(settingsResources['en-US'].services.summary, {
  requestTimeout: 'Single model-call timeout',
  requestTimeoutDescription: 'Maximum time for each model call, including prompt reading and output. Applies to external APIs, Ollama, and Built-in AI from the next summary task. Long summaries may make several calls; the Stop button remains available.',
  requestTimeoutMinutes: 'minutes',
  requestTimeoutSave: 'Save timeout',
  requestTimeoutSaving: 'Saving…',
  requestTimeoutSaved: 'Summary timeout saved',
  requestTimeoutLoadFailed: 'Could not load the summary timeout',
  requestTimeoutSaveFailed: 'Could not save the summary timeout',
  requestTimeoutInvalid: 'Enter a whole number from 5 to 1,440 minutes.',
});
Object.assign(settingsResources['zh-CN'].services.summary, {
  requestTimeout: '单次模型调用超时',
  requestTimeoutDescription: '每次模型调用读取提示词和生成输出的最长时间，从下一个摘要任务开始对外部 API、Ollama 和内置 AI 生效。长摘要可能包含多次调用，期间始终可以点击“停止”。',
  requestTimeoutMinutes: '分钟',
  requestTimeoutSave: '保存超时设置',
  requestTimeoutSaving: '保存中…',
  requestTimeoutSaved: '摘要超时设置已保存',
  requestTimeoutLoadFailed: '摘要超时设置加载失败',
  requestTimeoutSaveFailed: '摘要超时设置保存失败',
  requestTimeoutInvalid: '请输入 5–1440 之间的整数分钟。',
});

Object.assign(settingsResources['en-US'].pipeline, {
  loading: 'Loading transcription pipeline…',
  loadFailed: 'Could not load the transcription pipeline',
  finalizedModel: 'Saved transcript model',
  language: 'Transcription language',
  noContinuousPreview: 'No continuous preview',
  liveSpeakers: 'Live speaker labels',
  actualPipeline: 'What will actually run',
  resolvedLive: 'Live path: {{mode}}',
  speakerWillRun: 'Speaker refinement will run as a recoverable background task.',
  speakerWillNotRun: 'Speaker refinement will not run for this configuration.',
  presetDescriptions: {
    fast: 'Lowest overhead. VAD transcription only; no automatic post-meeting work.',
    balanced: 'VAD transcription with background speaker refinement; no ASR recompute.',
    quality: 'Keeps a meeting-time transcript, then automatically recomputes it with a larger ASR model.',
    custom: 'Choose every live, finalized, enhancement, and resource setting.',
  },
});
Object.assign(settingsResources['zh-CN'].pipeline, {
  loading: '正在加载转写 Pipeline…',
  loadFailed: '转写 Pipeline 加载失败',
  finalizedModel: '最终保存模型',
  language: '转写语言',
  noContinuousPreview: '不启用连续临时字幕',
  liveSpeakers: '实时说话人标签',
  actualPipeline: '实际将运行的 Pipeline',
  resolvedLive: '实时路径：{{mode}}',
  speakerWillRun: '说话人校正会作为可恢复的后台任务运行。',
  speakerWillNotRun: '当前配置不会运行说话人校正。',
  presetDescriptions: {
    fast: '资源占用最低，仅做 VAD 转写，不自动执行会后任务。',
    balanced: 'VAD 转写并在后台校正说话人，不进行 ASR 重算。',
    quality: '会议中保留可用稿，会后自动使用更大的 ASR 模型重新计算。',
    custom: '自行配置实时、最终、增强和资源控制的每一项。',
  },
});
Object.assign(settingsResources['en-US'].pipeline, {
  recommendedSetup: 'Recommended setup',
  recommendationDescriptions: {
    fast: 'Chinese-first, smallest download and lowest runtime cost. Uses Paraformer Small only.',
    balanced: 'SenseVoice for transcription, plus recoverable speaker refinement and Chinese/English punctuation.',
    quality: 'SenseVoice produces the meeting-time transcript; Fun-ASR Nano recomputes it after the meeting with terminology prompts, speaker refinement, and punctuation.',
  },
  assetKinds: { asr: 'ASR', speaker: 'Speakers', punctuation: 'Punctuation' },
  assetReady: 'Ready',
  assetMissing: 'Not installed',
  assetRepair: 'Needs repair',
  downloadRecommended: 'Download recommended setup',
  downloadingRecommended: 'Downloading recommended setup…',
  recommendedReady: 'All recommended models are ready',
  recommendedDownloadFailed: 'Could not download the recommended setup',
  downloadToActivate: 'Download the missing recommended models before saving this profile.',
  experimentalEnableFailed: 'Could not enable the high-quality model',
  resourceDescriptions: {
    eco: '1,024 MiB default limit; model threads use at most half of the logical CPU cores.',
    balanced: '2,048 MiB default limit; model threads may use all logical CPU cores.',
    fast: '4,096 MiB default limit; allows the largest models and full CPU thread budget.',
  },
});
Object.assign(settingsResources['zh-CN'].pipeline, {
  recommendedSetup: '推荐配置',
  recommendationDescriptions: {
    fast: '中文优先，下载最小、运行开销最低，仅使用 Paraformer Small。',
    balanced: '使用 SenseVoice 转写，并配套可恢复的说话人校正和中英文标点恢复。',
    quality: '会议中由 SenseVoice 生成可用稿；会后使用支持术语提示的 Fun-ASR Nano 重算，并进行说话人校正和标点处理。',
  },
  assetKinds: { asr: '语音识别', speaker: '说话人', punctuation: '标点' },
  assetReady: '已就绪',
  assetMissing: '未安装',
  assetRepair: '需要修复',
  downloadRecommended: '下载推荐配置',
  downloadingRecommended: '正在下载推荐配置…',
  recommendedReady: '推荐模型均已就绪',
  recommendedDownloadFailed: '推荐配置下载失败',
  downloadToActivate: '请先下载缺失的推荐模型，再保存该方案。',
  experimentalEnableFailed: '无法启用高质量模型',
  resourceDescriptions: {
    eco: '默认内存上限 1024 MiB；模型线程最多使用一半逻辑 CPU 核心。',
    balanced: '默认内存上限 2048 MiB；模型线程可使用全部逻辑 CPU 核心。',
    fast: '默认内存上限 4096 MiB；允许最大模型和完整 CPU 线程预算。',
  },
});
Object.assign(settingsResources['en-US'].pipelineDecisions, {
  streamingStableFallback: 'Experimental live preview is gated off; VAD-segmented transcription will be used.',
  postMeetingStableFallback: 'The experimental post-meeting model is gated off; SenseVoice will be used for this run.',
  speakerModelUnavailable: 'Speaker refinement is disabled because its local model is missing or damaged.',
  damagedFinalizedModelFallback: 'The selected finalized model is missing or damaged; SenseVoice will run without changing your saved selection.',
  damagedStreamingModelFallback: 'The selected streaming model is missing or damaged; continuous preview is disabled.',
  damagedPostMeetingModelFallback: 'The selected post-meeting model is missing or damaged; SenseVoice will be used instead.',
  nativeSpeakerOutput: 'Independent speaker diarization is skipped because the ASR model already returns speaker labels.',
});
Object.assign(settingsResources['en-US'].beta, {
  saveFailed: 'Could not save the Beta setting. The previous value was restored.',
});
Object.assign(settingsResources['zh-CN'].beta, {
  saveFailed: 'Beta 设置保存失败，已恢复之前的状态。',
});
Object.assign(settingsResources['en-US'].pipeline.jobs, {
  title: 'Meeting processing',
  retry: 'Retry',
  actionFailed: 'Could not update the processing job',
  etaMinutes: 'about {{count}} min left',
});
Object.assign(settingsResources['zh-CN'].pipeline.jobs, {
  title: '会议后台处理',
  retry: '重试',
  actionFailed: '后台任务操作失败',
  etaMinutes: '预计剩余 {{count}} 分钟',
});
Object.assign(settingsResources['zh-CN'].pipelineDecisions, {
  streamingStableFallback: '实验性实时模型开关已关闭，实际使用 VAD 分段转写。',
  postMeetingStableFallback: '实验性会后模型开关已关闭，本次改用 SenseVoice。',
  speakerModelUnavailable: '说话人模型缺失或损坏，因此不会启动会后说话人校正。',
  damagedFinalizedModelFallback: '已选最终模型缺失或损坏，实际改用 SenseVoice，原选择保持不变。',
  damagedStreamingModelFallback: '已选流式模型缺失或损坏，实际关闭连续临时字幕。',
  damagedPostMeetingModelFallback: '已选会后模型缺失或损坏，实际改用 SenseVoice。',
  nativeSpeakerOutput: '当前 ASR 模型已输出说话人标签，因此不会再启动独立说话人分离。',
});
Object.assign(settingsResources['en-US'].services.transcription.terminology, {
  customTitle: 'Custom terminology',
  customDescription: 'Improve names and domain terms across recording, import, and retranscription without learning ASR internals.',
  terms: 'Terms and names',
  termCount: '{{count}} unique terms · {{characters}} characters',
  tooManyTerms: 'Use no more than 200 terms.',
  termTooLong: 'Each term must be 100 characters or fewer.',
  termsTooLong: 'Terms can contain at most 4,000 characters in total.',
  nextSession: 'Applies to the next recording, import, or retranscription',
  whisperBehavior: 'Whisper receives these terms as an initial prompt. Exact corrections are then applied to saved text.',
  promptBehavior: 'The selected model receives these terms as a recognition prompt. Exact corrections are then applied to saved text.',
  correctionOnlyBehavior: 'This model does not support recognition prompts. Exact corrections still apply to saved finalized text.',
  replacements: 'Exact corrections',
  replacementsHint: 'Case-sensitive literal matching. Longer sources win and replacements do not cascade.',
  addReplacement: 'Add correction',
  sourcePlaceholder: 'Recognized as',
  targetPlaceholder: 'Replace with',
  removeReplacement: 'Remove correction',
  advancedCompatibility: 'Advanced compatibility options',
  advancedCompatibilityHint: 'For existing Sherpa lexicon and pre-generated FST workflows. Mingtily does not generate FST files or install Pynini.',
  manageAdvancedResources: 'Manage advanced resources in Models',
  saved: 'Custom terminology saved',
  saveFailed: 'Could not save custom terminology',
});
Object.assign(settingsResources['zh-CN'].services.transcription.terminology, {
  customTitle: '自定义术语',
  customDescription: '无需理解 ASR 内部格式，即可统一改善录音、导入和重新转写中的姓名与领域术语。',
  terms: '术语与姓名',
  termCount: '{{count}} 个去重术语 · {{characters}} 个字符',
  tooManyTerms: '术语不能超过 200 个。',
  termTooLong: '每个术语不能超过 100 个字符。',
  termsTooLong: '全部术语合计不能超过 4,000 个字符。',
  nextSession: '下次录音、导入或重新转写生效',
  whisperBehavior: 'Whisper 会将术语作为初始提示词，并对最终保存文本应用精确纠错。',
  promptBehavior: '当前模型会将术语作为识别提示词，并对最终保存文本应用精确纠错。',
  correctionOnlyBehavior: '当前模型不支持识别提示词，但仍会对最终保存文本应用精确纠错。',
  replacements: '精确纠错',
  replacementsHint: '区分大小写的字面匹配；较长原文优先，替换结果不会再次匹配。',
  addReplacement: '添加纠错',
  sourcePlaceholder: '识别成',
  targetPlaceholder: '替换为',
  removeReplacement: '删除纠错',
  advancedCompatibility: '高级兼容选项',
  advancedCompatibilityHint: '仅用于已有 Sherpa 词典和预生成 FST 的工作流。Mingtily 不生成 FST，也不会安装 Pynini。',
  manageAdvancedResources: '在“模型”中管理高级资源',
  saved: '自定义术语已保存',
  saveFailed: '无法保存自定义术语',
});

for (const locale of ['en-US', 'zh-CN'] as const) {
  Object.assign(
    settingsResources[locale].recordings,
    settingsExtraResources[locale].settings.recordings
  );
  Object.assign(
    settingsResources[locale].services.transcription,
    settingsExtraResources[locale].settings.services.transcription
  );
  Object.assign(
    settingsResources[locale].services.summary,
    settingsExtraResources[locale].settings.services.summary
  );
}
