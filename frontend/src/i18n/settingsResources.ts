import { settingsExtraResources } from './settingsExtraResources';

export const settingsResources = {
  'en-US': {
    ...settingsExtraResources['en-US'].settings,
    title: 'Settings', back: 'Back',
    tabs: { general: 'General', recordings: 'Recordings', models: 'Models', services: 'Services', beta: 'Beta' },
    actions: { save: 'Save', saving: 'Saving…' },
    general: { loading: 'Loading preferences…', appLanguage: 'App language', appLanguageDescription: 'Changes the interface immediately. Transcription language is configured separately.', notifications: 'Notifications', notificationsDescription: 'Notify me when a meeting recording starts or ends.', storage: 'Data storage', storageDescription: 'View where Mingtily keeps local data.', recordingsFolder: 'Meeting recordings', openFolder: 'Open folder', noteLabel: 'Note:', storageNote: 'The database and models share Mingtily’s app-data directory.', diagnostics: 'Diagnostic logs', diagnosticsDescription: 'Mingtily keeps up to five local 5 MB log files to help investigate recording, model, and startup problems.', diagnosticsPrivacy: 'Logs are never uploaded automatically. Exporting is optional and replaces your home-directory path and obvious credential-bearing lines.', exportDiagnostics: 'Export diagnostics', exportingDiagnostics: 'Exporting…', diagnosticsExported: 'Diagnostics exported', diagnosticsExportedDescription: '{{count}} log file(s) included.', diagnosticsExportFailed: 'Could not export diagnostics', updates: { title: 'App updates', description: 'Automatically check GitHub Releases after startup once you enable this option.', privacy: 'Disabled by default. Enabling it contacts github.com; no transcript or meeting data is sent.', idle: 'Automatic checks are optional.', developmentDisabled: 'Updates are disabled in Mingtily Dev.', checking: 'Checking for updates…', checkNow: 'Check now', current: 'Mingtily is up to date.', available: 'Mingtily {{version}} is available.', download: 'Download update', downloading: 'Downloading Mingtily {{version}}…', downloadingProgress: 'Downloading Mingtily {{version}}… {{progress}}%', ready: 'Mingtily {{version}} is installed and ready to restart.', restartNow: 'Restart now', restartBlocked: 'Stop the current recording before restarting to update.', restartFailed: 'Could not restart Mingtily.', checkFailed: 'Could not check for updates.', installFailed: 'Could not install the update.' } },
    recordings: { title: 'Recording settings', description: 'Choose how meeting audio is saved and which devices are used by default.', saveAudio: 'Save audio recordings', saveAudioDescription: 'Keep the audio file after recording stops.', saveLocation: 'Save location', defaultFolder: 'Default folder', fileFormat: 'File format:', filenamePattern: 'Files use the name recording_YYYYMMDD_HHMMSS.{{extension}}', disabledHint: 'Audio saving is off. Turn on “Save audio recordings” to keep meeting audio.', startNotification: 'Recording reminder', startNotificationDescription: 'Remind you to notify participants when recording begins.', defaultDevices: 'Default audio devices', defaultDevicesDescription: 'Mingtily will preselect these microphone and system-audio devices for new recordings.', preferenceSaved: 'Preference saved', preferenceSaveFailed: 'Could not save preference', defaultDevice: 'Default', devicesSaved: 'Audio devices saved', devicesSavedDescription: 'Microphone: {{mic}} · System audio: {{system}}', devicesSaveFailed: 'Could not save audio devices' },
    beta: { title: 'Beta features', description: 'These features are still being tested and may have rough edges.', features: { importAndRetranscribe: { name: 'Import & retranscribe', description: 'Import audio as a new meeting or run transcription again with different settings.' } }, note: 'Turning a feature off only hides its entry points. Existing meetings are not changed.' },
    services: {
      provider: 'Provider', model: 'Model', selectProvider: 'Select provider', selectModel: 'Select model…', selectInstalledModel: 'Select an installed model', noInstalledModel: 'No installed model is available for this provider.', searchModels: 'Search models…', loadingModels: 'Loading models…', noModels: 'No models found.', fetchingModels: 'Fetching…', fetchModels: 'Fetch models',
      transcription: { title: 'Speech recognition', description: 'Choose the local ASR provider, installed model, and default audio language.', mode: 'Recognition mode', modes: { stable: 'Stable · single model', betaLive: 'Beta · live enhancement' }, stableModeDescription: 'One model transcribes finalized VAD segments. This uses less memory and compute.', betaModeDescription: 'A streaming model shows immediate provisional text while a separate finalized model produces the saved transcript.', streamingPath: 'Live provisional path', streamingProvider: 'Streaming provider', streamingModel: 'Streaming model', streamingModelMissing: 'No continuous streaming model is installed. Download one in Models.', finalizedPath: 'Saved finalized path', finalizedProvider: 'Finalized provider', finalizedModel: 'Finalized model', saved: 'Speech recognition service saved', saveFailed: 'Could not save speech recognition service', language: 'Transcription language', autoDetect: 'Auto-detect (original language)', autoTranslate: 'Auto-detect and translate to English', languageSaved: 'Transcription language saved', languageSavedDescription: 'New recordings will use {{language}}.', languageSaveFailed: 'Could not save transcription language', parakeetLanguageTitle: 'Parakeet language support', parakeetLanguageDescription: 'Parakeet is English-only and does not support Chinese. Choose Whisper or a Sherpa ONNX model for Chinese and multilingual audio.', automaticLanguageTitle: 'Automatic language detection', automaticLanguageDescription: '{{model}} detects the language automatically and does not accept a fixed language.', currentLanguage: 'Current:', autoDetectWarning: 'Automatic detection can choose the wrong language', autoDetectHint: 'For predictable results, choose the language spoken in the recording.', translationActive: 'English translation is on', translationDescription: 'Speech in any supported language will be translated into English.', optimizedFor: 'Transcription will be optimized for', streamingNotice: 'Provisional streaming hypotheses are display-only and may change. Only finalized VAD segments are saved to the meeting.', terminology: { title: 'Finalized terminology enhancement', description: 'Optional local enhancements applied only to saved finalized segments. Hotwords can be prepared here before switching to a supported model.', hotwords: 'Dynamic hotwords', hotwordsPlaceholder: 'One term per line, for example:\nMingtily\nSenseVoice\nQwen3-ASR', hotwordsHint: 'Qwen3-ASR and FunASR Nano receive these terms at recognizer initialization. Separate terms with a new line or comma.', hotwordsUnsupported: 'The selected finalized model does not support dynamic hotwords. The terms are still saved and will apply after switching to Qwen3-ASR or FunASR Nano.', homophone: 'Chinese homophone replacement', homophoneHint: 'Uses Sherpa’s local lexicon and selected pre-generated .fst rules after recognition. It only changes Chinese characters.', homophoneSherpaOnly: 'Chinese homophone replacement is available when the finalized Provider is Sherpa ONNX. Hotwords above can be configured in advance for Qwen3-ASR or FunASR Nano.', lexiconMissing: 'Download the Chinese homophone lexicon in Models before enabling this option.', rulesMissing: 'Import at least one pre-generated .fst rule in Models before enabling this option.', selectRules: 'Rules used for finalized text' } },
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
    beta: { title: 'Beta 功能', description: '这些功能仍在测试，使用中可能遇到不完善之处。', features: { importAndRetranscribe: { name: '导入与重新转写', description: '导入音频创建会议，或使用不同设置重新生成已有会议的转写。' } }, note: '关闭后只会隐藏相关入口，不影响已有会议。' },
    services: {
      provider: 'Provider', model: '模型', selectProvider: '选择 Provider', selectModel: '选择模型…', selectInstalledModel: '选择已安装模型', noInstalledModel: '该 Provider 暂无可用的已安装模型。', searchModels: '搜索模型…', loadingModels: '正在加载模型…', noModels: '没有找到模型。', fetchingModels: '正在获取…', fetchModels: '获取模型',
      transcription: { title: '语音转写', description: '选择本地 ASR Provider、已安装模型和默认音频语言。', mode: '识别模式', modes: { stable: '稳定模式 · 单模型', betaLive: 'Beta · 实时增强' }, stableModeDescription: '使用一个模型处理 VAD 完成后的最终片段，占用的内存和算力更少。', betaModeDescription: '流式模型即时展示临时文本，另一套最终模型生成并保存正式转写。', streamingPath: '实时临时识别', streamingProvider: '流式 Provider', streamingModel: '流式模型', streamingModelMissing: '尚未安装支持连续流式识别的模型，请前往“模型”下载。', finalizedPath: '正式转写', finalizedProvider: '最终 Provider', finalizedModel: '最终模型', saved: '语音转写服务已保存', saveFailed: '语音转写服务保存失败', language: '转写语言', autoDetect: '自动检测（保留原语言）', autoTranslate: '自动检测并翻译为英文', languageSaved: '转写语言已保存', languageSavedDescription: '新录音将使用：{{language}}。', languageSaveFailed: '转写语言保存失败', parakeetLanguageTitle: 'Parakeet 语言支持', parakeetLanguageDescription: 'Parakeet 仅支持英文，不支持中文。中文或多语言录音请使用 Whisper 或 Sherpa ONNX 模型。', automaticLanguageTitle: '自动判断语言', automaticLanguageDescription: '{{model}} 会自动判断语言，暂不接受固定语言设置。', currentLanguage: '当前：', autoDetectWarning: '自动检测可能选错语言', autoDetectHint: '如需稳定结果，建议直接选择录音中使用的语言。', translationActive: '英文翻译已开启', translationDescription: '支持的语音内容会统一翻译为英文。', optimizedFor: '转写将针对以下语言优化：', streamingNotice: '流式临时结果只用于展示，内容可能随识别修订；会议中只保存最终模型处理完成的 VAD 片段。', terminology: { title: '最终转写术语增强', description: '这些可选的本地增强只作用于保存的最终片段。可以先在这里配置热词，再切换到支持的模型。', hotwords: '动态热词', hotwordsPlaceholder: '每行一个术语，例如：\nMingtily\nSenseVoice\nQwen3-ASR', hotwordsHint: 'Qwen3-ASR 和 FunASR Nano 会在初始化识别器时接收这些术语；可使用换行或逗号分隔。', hotwordsUnsupported: '当前最终模型不支持动态热词，但术语仍会保存；切换到 Qwen3-ASR 或 FunASR Nano 后生效。', homophone: '中文同音词替换', homophoneHint: '识别完成后使用 Sherpa 本地词典和选中的预生成 .fst 规则，只处理中文汉字。', homophoneSherpaOnly: '中文同音词替换仅在最终 Provider 为 Sherpa ONNX 时可用。上面的热词可以提前配置，切换到 Qwen3-ASR 或 FunASR Nano 后即可使用。', lexiconMissing: '请先在“模型”中下载中文同音词词典，再开启此功能。', rulesMissing: '请先在“模型”中导入至少一个预生成 .fst 规则，再开启此功能。', selectRules: '用于最终文本的规则' } },
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
