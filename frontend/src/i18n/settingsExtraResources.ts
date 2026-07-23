export const settingsExtraResources = {
  'en-US': {
    settings: {
      legacyTabs: {
        transcription: 'Transcription', summary: 'AI Summary', preferences: 'Preferences', about: 'About',
      },
      recordings: {
        devices: {
          title: 'Audio devices', loadFailed: 'Could not load audio devices. Check your system audio settings.', noMicrophonesToTest: 'No microphone is available to test.', monitoringFailed: 'Could not start microphone level monitoring.', microphone: 'Microphone', selectMicrophone: 'Select microphone', defaultMicrophone: 'Default microphone', noMicrophones: 'No microphones found', microphoneLevels: 'Microphone levels', systemAudio: 'System audio', selectSystemAudio: 'Select system audio', defaultSystemAudio: 'Default system audio', noSystemAudio: 'No system-audio devices found', microphoneHint: 'Records your voice and nearby sound.', systemAudioHint: 'Records audio from your computer, such as calls and media.', levelsLabel: 'Levels', levelsHint: 'Green is good, yellow is loud, and red is too loud.', tipLabel: 'Tip', testHint: 'Test the microphone before an important meeting.',
        },
        backend: {
          title: 'System-audio backend', loadFailed: 'Could not load audio backend options.', changeFailed: 'Could not change the audio backend. Try again.', captureMethods: 'Audio capture methods', tryDifferent: 'If capture is unreliable, try another backend.', active: 'Active', disabled: 'Unavailable', systemOnly: 'This setting only affects system-audio capture.', microphoneDefault: 'The microphone always uses the default capture method.', nextSession: 'Changes take effect when the next recording starts.',
        },
      },
      services: {
        transcription: {
          modelLabel: 'Transcription model', parakeetOption: 'Parakeet — recommended for real-time speed and accuracy', whisperOption: 'Local Whisper — high accuracy',
        },
        summary: {
          showApiKey: 'Show API key', hideApiKey: 'Hide API key', invalidOllamaEndpoint: 'Enter a valid Ollama URL starting with http:// or https://.', ollamaLoadFailed: 'Could not load Ollama models.', openRouterLoadFailed: 'Could not load OpenRouter models.', builtinLoadFailed: 'Could not load Built-in AI models.', customOpenAISaveFailed: 'Could not save the OpenAI-compatible service.', endpointAndModelRequired: 'Enter the endpoint URL and model name first.', connectionSuccessful: 'Connection successful', maxTokens: 'Max tokens', temperature: 'Temperature (0.0–2.0)', topP: 'Top P (0.0–1.0)', modelNameExample: 'For example: gpt-4 or llama-3-70b', exampleValue: 'For example: {{value}}',
        },
      },
      modal: {
        preferences: 'Preferences', aiModelConfiguration: 'AI model configuration', builtinAi: 'Built-in AI', availableOllamaModels: 'Available Ollama models', modelSize: 'Size: {{size}}', modelModified: 'Modified: {{modified}}', audioDevices: 'Audio device settings', devicesSelected: 'Audio devices selected', language: 'Language settings', speechSetupRequired: 'Speech recognition setup required', transcriptionModels: 'Transcription model settings', confidenceIndicators: 'Show confidence indicators', confidenceDescription: 'Use colored dots to show transcription confidence.', recordingStopped: 'Recording stopped', transcriptionWarning: 'Transcription performance warning', dismiss: 'Dismiss',
      },
    },
    models: {
      download: {
        alreadyDownloading: '{{model}} is already downloading', progressLabel: 'Progress: {{progress}}%', downloadingModel: 'Downloading {{model}}…',
      },
      ollama: {
        notInstalled: 'Ollama is not installed', installBeforeDownload: 'Install and start Ollama before downloading models.', availableModels: 'Available Ollama models', usingEndpoint: 'Endpoint:', notRunning: 'Ollama is not installed or is not running.', downloadOllama: 'Download Ollama', restartAfterInstall: 'After installation, restart Mingtily and fetch models again.', endpointChanged: 'Fetch models from the new endpoint.', noModelsHint: 'No models found. Pull a recommended model or fetch again.', downloadRecommended: 'Download {{model}} — recommended, about {{size}}', noSearchResults: 'No models match “{{query}}”.', modelSizeConnector: ' · size ',
      },
    },
  },
  'zh-CN': {
    settings: {
      legacyTabs: {
        transcription: '转写', summary: 'AI 总结', preferences: '偏好设置', about: '关于',
      },
      recordings: {
        devices: {
          title: '音频设备', loadFailed: '音频设备加载失败，请检查系统音频设置。', noMicrophonesToTest: '没有可测试的麦克风。', monitoringFailed: '无法启动麦克风音量检测。', microphone: '麦克风', selectMicrophone: '选择麦克风', defaultMicrophone: '默认麦克风', noMicrophones: '未找到麦克风', microphoneLevels: '麦克风音量', systemAudio: '系统音频', selectSystemAudio: '选择系统音频', defaultSystemAudio: '默认系统音频', noSystemAudio: '未找到系统音频设备', microphoneHint: '录制你的声音和周围环境音。', systemAudioHint: '录制通话、媒体等电脑声音。', levelsLabel: '音量', levelsHint: '绿色正常，黄色偏响，红色表示音量过大。', tipLabel: '提示', testHint: '重要会议前建议先测试麦克风。',
        },
        backend: {
          title: '系统音频后端', loadFailed: '音频后端选项加载失败。', changeFailed: '音频后端切换失败，请重试。', captureMethods: '音频采集方式', tryDifferent: '采集不稳定时，可以尝试其他后端。', active: '当前使用', disabled: '不可用', systemOnly: '此设置仅影响系统音频采集。', microphoneDefault: '麦克风始终使用默认采集方式。', nextSession: '更改会在下次开始录音时生效。',
        },
      },
      services: {
        transcription: {
          modelLabel: '转写模型', parakeetOption: 'Parakeet — 推荐，实时且准确', whisperOption: '本地 Whisper — 高准确度',
        },
        summary: {
          showApiKey: '显示 API Key', hideApiKey: '隐藏 API Key', invalidOllamaEndpoint: '请输入以 http:// 或 https:// 开头的有效 Ollama 地址。', ollamaLoadFailed: 'Ollama 模型加载失败。', openRouterLoadFailed: 'OpenRouter 模型加载失败。', builtinLoadFailed: '内置 AI 模型加载失败。', customOpenAISaveFailed: 'OpenAI 兼容服务保存失败。', endpointAndModelRequired: '请先填写 Endpoint URL 和模型名称。', connectionSuccessful: '连接成功', maxTokens: '最大 Token 数', temperature: 'Temperature（0.0–2.0）', topP: 'Top P（0.0–1.0）', modelNameExample: '例如：gpt-4 或 llama-3-70b', exampleValue: '例如：{{value}}',
        },
      },
      modal: {
        preferences: '偏好设置', aiModelConfiguration: 'AI 模型配置', builtinAi: '内置 AI', availableOllamaModels: '可用的 Ollama 模型', modelSize: '大小：{{size}}', modelModified: '更新时间：{{modified}}', audioDevices: '音频设备设置', devicesSelected: '音频设备已选择', language: '语言设置', speechSetupRequired: '需要配置语音转写', transcriptionModels: '转写模型设置', confidenceIndicators: '显示置信度标记', confidenceDescription: '使用彩色圆点显示转写置信度。', recordingStopped: '录音已停止', transcriptionWarning: '转写性能提醒', dismiss: '知道了',
      },
    },
    models: {
      download: {
        alreadyDownloading: '{{model}} 已在下载中', progressLabel: '进度：{{progress}}%', downloadingModel: '正在下载 {{model}}…',
      },
      ollama: {
        notInstalled: '尚未安装 Ollama', installBeforeDownload: '请先安装并启动 Ollama，再下载模型。', availableModels: '可用的 Ollama 模型', usingEndpoint: 'Endpoint：', notRunning: 'Ollama 尚未安装或未运行。', downloadOllama: '下载 Ollama', restartAfterInstall: '安装完成后请重启 Mingtily，再重新获取模型。', endpointChanged: '请从新的 Endpoint 获取模型。', noModelsHint: '没有找到模型，可以拉取推荐模型或重新获取。', downloadRecommended: '下载 {{model}} — 推荐，约 {{size}}', noSearchResults: '没有与“{{query}}”匹配的模型。', modelSizeConnector: ' · 大小 ',
      },
    },
  },
} as const;
