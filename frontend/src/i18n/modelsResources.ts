import { settingsExtraResources } from './settingsExtraResources';

export const modelsResources = {
  'en-US': {
    ...settingsExtraResources['en-US'].models,
    sections: { transcription: 'Speech recognition models', transcriptionDescription: 'Manage local models for Chinese and multilingual meetings. SenseVoice is recommended for Chinese; choose the active model in Services.', speaker: 'Speaker diarization model', speakerDescription: 'Manage the local segmentation and speaker-embedding package.', localSummary: 'Built-in summary models', localSummaryDescription: 'Manage offline GGUF models used by Built-in AI.', ollama: 'Ollama models', ollamaDescription: 'Scan, pull, and remove models from the configured Ollama service.', advanced: 'More Whisper models' },
    actions: { download: 'Download', retry: 'Retry', repair: 'Repair', delete: 'Delete', use: 'Use model', redownload: 'Download again', cancel: 'Cancel', cancelDownload: 'Cancel download', refresh: 'Refresh' },
    status: { ready: 'Ready', installed: 'Installed', notInstalled: 'Not installed', needsRepair: 'Needs repair', inUse: 'In use', recommended: 'Recommended', downloading: 'Downloading…', loading: 'Loading models…', corrupted: 'Corrupted', error: 'Error' },
    download: { readyTitle: '{{icon}} {{model}} is ready', readyDescription: 'Downloaded and ready to use.', failed: 'Could not download {{model}}', genericFailed: 'Download failed', cancelled: '{{model}} download cancelled', cancelFailed: 'Could not cancel download', starting: 'Downloading {{model}}…', mayTakeMinutes: 'This may take a few minutes.', completed: '{{model}} downloaded', progress: 'Downloading… {{progress}}%' },
    selection: { switched: 'Switched to {{model}}', usingForTranscription: 'Using {{model}} for transcription' },
    delete: { deleted: '{{model}} deleted', freedSpace: 'The model files were removed.', failed: 'Could not delete {{model}}', genericFailed: 'Delete failed', freeSpace: 'Delete model to free storage', activeBlocked: 'Choose another model in Services before deleting this one.', disableSpeakerFirst: 'Turn off speaker diarization in Services before deleting its model.' },
    errors: { loadModels: 'Could not load models', loadTranscriptionModels: 'Could not load speech recognition models', unknown: 'Unknown error', corruptedModel: 'The file is corrupted. Retry the download or delete it.', generic: 'Something went wrong.', pageFailed: 'Could not display model settings', pageFailedDescription: 'A model component failed to render. Retry after reviewing the error below.' },
    specs: { accuracy: '{{accuracy}} accuracy', processing: '{{speed}} processing' },
    metrics: {
      accuracy: { high: 'High', good: 'Good', decent: 'Decent' },
      speed: { slow: 'Slow', medium: 'Medium', fast: 'Fast', veryFast: 'Very fast', ultraFast: 'Ultra fast' },
    },
    precision: { full: 'Full precision', balancedPlus: 'Balanced+', balanced: 'Balanced', fast: 'Fast', standard: 'Standard' },
    whisper: {
      taglines: {
        largeV3: 'Slower processing · Most accurate',
        largeV3Turbo: 'Moderate speed · Best accuracy with speed',
        medium: 'Slower processing · Professional quality',
        small: 'Moderate speed · Good accuracy',
        base: 'Fast processing · Balanced quality',
        tiny: 'Real time · Fastest option',
      },
      optimizedSuffix: ' · Optimized',
    },
    parakeet: {
      lightning: { name: 'Lightning', tagline: 'Real time · Best for speed with great accuracy' },
      compact: { name: 'Compact', tagline: 'Real time · Smaller download' },
      precise: { name: 'Precise', tagline: '20× real time · Higher precision' },
    },
    sherpa: {
      models: {
        'sensevoice-small-int8': { description: 'Recommended for Chinese, Cantonese, English, Japanese, and Korean. Supports a fixed language and Chinese ITN.' },
        'paraformer-zh-small-int8': { description: 'Lightweight Chinese and English model. Uses automatic language detection after each VAD segment.' },
        'qwen3-asr-0.6b-int8': { description: 'High-quality multilingual and dialect model. Beta because it needs substantially more memory and compute.' },
      },
      sizeAndLicense: '{{download}} download · {{installed}} installed · {{license}}',
      downloaded: '{{model}} is ready',
      removed: '{{model}} removed',
      errors: { load: 'Could not load Sherpa ONNX models', download: 'Could not download the Sherpa ONNX model', delete: 'Could not delete the Sherpa ONNX model' },
    },
    builtInMetadata: {
      qwen2b: { name: 'Qwen 3.5 2B (Balanced)', description: 'Balanced local summary model with strong quality and modest hardware requirements.' },
      qwen4b: { name: 'Qwen 3.5 4B (High quality)', description: 'The highest-quality local Qwen option currently available in Mingtily.' },
      gemma4b: { name: 'Gemma 3 4B (Balanced)', description: 'A balanced quality-and-speed option that needs about 3.5 GB of memory.' },
      gemma1b: { name: 'Gemma 3 1B (Fast)', description: 'The fastest built-in option; runs on most hardware with about 1 GB of memory.' },
      contextTokens: '{{count}} context tokens',
    },
    speaker: { title: 'Speaker diarization', description: 'Pyannote segmentation + 3D-Speaker ERes2Net · about {{size}} MB', ready: 'Speaker diarization is ready', downloadFailed: 'Could not download speaker model', removed: 'Speaker model removed', removeFailed: 'Could not remove speaker model' },
    builtin: { title: 'Built-in AI models', empty: 'No built-in models found. Download one to use Built-in AI.' },
    ollama: { modelPlaceholder: 'Model name, for example qwen3:8b', pull: 'Pull model', pullComplete: '{{model}} is ready', pullFailed: 'Could not pull Ollama model', deleted: '{{model}} deleted', deleteFailed: 'Could not delete Ollama model', endpoint: 'Ollama endpoint: {{endpoint}}', unavailable: 'Ollama is unavailable', empty: 'No Ollama models found. Enter a model name above to pull one.' }
  },
  'zh-CN': {
    ...settingsExtraResources['zh-CN'].models,
    sections: { transcription: '语音转写模型', transcriptionDescription: '统一管理中文与多语言会议使用的本地模型；中文场景推荐 SenseVoice，当前使用的模型请在“服务”中选择。', speaker: '说话人分离模型', speakerDescription: '管理本地分割与说话人向量模型包。', localSummary: '内置总结模型', localSummaryDescription: '管理内置 AI 使用的离线 GGUF 模型。', ollama: 'Ollama 模型', ollamaDescription: '扫描、拉取或删除当前 Ollama 服务中的模型。', advanced: '更多 Whisper 模型' },
    actions: { download: '下载', retry: '重试', repair: '修复', delete: '删除', use: '使用此模型', redownload: '重新下载', cancel: '取消', cancelDownload: '取消下载', refresh: '刷新' },
    status: { ready: '可用', installed: '已安装', notInstalled: '未安装', needsRepair: '需要修复', inUse: '正在使用', recommended: '推荐', downloading: '正在下载…', loading: '正在加载模型…', corrupted: '文件损坏', error: '错误' },
    download: { readyTitle: '{{icon}} {{model}} 已可用', readyDescription: '模型已下载，可以开始使用。', failed: '{{model}} 下载失败', genericFailed: '下载失败', cancelled: '已取消下载 {{model}}', cancelFailed: '取消下载失败', starting: '正在下载 {{model}}…', mayTakeMinutes: '下载可能需要几分钟。', completed: '{{model}} 下载完成', progress: '正在下载… {{progress}}%' },
    selection: { switched: '已切换到 {{model}}', usingForTranscription: '当前使用 {{model}} 转写' },
    delete: { deleted: '{{model}} 已删除', freedSpace: '模型文件已移除。', failed: '{{model}} 删除失败', genericFailed: '删除失败', freeSpace: '删除模型以释放空间', activeBlocked: '请先在“服务”中改用其他模型。', disableSpeakerFirst: '请先在“服务”中关闭说话人分离。' },
    errors: { loadModels: '模型加载失败', loadTranscriptionModels: '语音转写模型加载失败', unknown: '未知错误', corruptedModel: '模型文件已损坏，请重新下载或删除。', generic: '发生错误。', pageFailed: '模型设置显示失败', pageFailedDescription: '某个模型组件渲染失败。请根据下方错误信息检查后重试。' },
    specs: { accuracy: '准确度：{{accuracy}}', processing: '处理速度：{{speed}}' },
    metrics: {
      accuracy: { high: '高', good: '良好', decent: '基础' },
      speed: { slow: '较慢', medium: '中等', fast: '快', veryFast: '很快', ultraFast: '极速' },
    },
    precision: { full: '全精度', balancedPlus: '均衡增强', balanced: '均衡', fast: '快速', standard: '标准' },
    whisper: {
      taglines: {
        largeV3: '处理较慢 · 准确度最高',
        largeV3Turbo: '速度中等 · 兼顾高准确度与速度',
        medium: '处理较慢 · 专业级质量',
        small: '速度中等 · 准确度良好',
        base: '处理快速 · 质量均衡',
        tiny: '接近实时 · 速度最快',
      },
      optimizedSuffix: ' · 已优化',
    },
    parakeet: {
      lightning: { name: '极速', tagline: '实时处理 · 速度优先且准确度良好' },
      compact: { name: '轻巧', tagline: '实时处理 · 下载体积更小' },
      precise: { name: '精准', tagline: '约 20 倍实时速度 · 更高精度' },
    },
    sherpa: {
      models: {
        'sensevoice-small-int8': { description: '中文推荐模型，支持普通话、粤语、英语、日语和韩语，可指定语言并启用中文 ITN。' },
        'paraformer-zh-small-int8': { description: '轻量中文、英文模型；每个 VAD 语音段完成后自动判断语言。' },
        'qwen3-asr-0.6b-int8': { description: '高质量多语言与方言模型；因内存和算力需求明显更高，暂列为 Beta。' },
      },
      sizeAndLicense: '下载 {{download}} · 安装后 {{installed}} · {{license}}',
      downloaded: '{{model}} 已可用',
      removed: '已删除 {{model}}',
      errors: { load: 'Sherpa ONNX 模型加载失败', download: 'Sherpa ONNX 模型下载失败', delete: 'Sherpa ONNX 模型删除失败' },
    },
    builtInMetadata: {
      qwen2b: { name: 'Qwen 3.5 2B（均衡）', description: '本地总结质量与硬件需求较均衡，适合大多数设备。' },
      qwen4b: { name: 'Qwen 3.5 4B（高质量）', description: '当前 Mingtily 中质量最高的本地 Qwen 模型。' },
      gemma4b: { name: 'Gemma 3 4B（均衡）', description: '兼顾质量与速度，约需 3.5 GB 内存。' },
      gemma1b: { name: 'Gemma 3 1B（快速）', description: '内置模型中速度最快，约 1 GB 内存即可运行。' },
      contextTokens: '{{count}} 个上下文 Token',
    },
    speaker: { title: '说话人分离', description: 'Pyannote 分割 + 3D-Speaker ERes2Net，约 {{size}} MB', ready: '说话人分离模型已可用', downloadFailed: '说话人分离模型下载失败', removed: '说话人分离模型已删除', removeFailed: '说话人分离模型删除失败' },
    builtin: { title: '内置 AI 模型', empty: '没有找到内置模型。请先下载一个模型。' },
    ollama: { modelPlaceholder: '模型名称，例如 qwen3:8b', pull: '拉取模型', pullComplete: '{{model}} 已可用', pullFailed: 'Ollama 模型拉取失败', deleted: '{{model}} 已删除', deleteFailed: 'Ollama 模型删除失败', endpoint: 'Ollama Endpoint：{{endpoint}}', unavailable: 'Ollama 当前不可用', empty: '没有找到 Ollama 模型，可在上方输入名称并拉取。' }
  }
} as const;

for (const locale of ['en-US', 'zh-CN'] as const) {
  Object.assign(
    modelsResources[locale].download,
    settingsExtraResources[locale].models.download
  );
  Object.assign(
    modelsResources[locale].ollama,
    settingsExtraResources[locale].models.ollama
  );
}
