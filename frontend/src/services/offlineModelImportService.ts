import { invoke } from '@tauri-apps/api/core';
import { notifyModelAssetsChanged } from '@/lib/model-assets-events';
import { SherpaAsrAPI } from '@/lib/sherpa-asr';

export interface ImportedOfflineModel {
  family: 'sherpa-onnx' | 'whisper';
  modelId: string;
  name: string;
  path: string;
}

export const offlineModelImportService = {
  importArchive: async (): Promise<ImportedOfflineModel | null> => {
    const imported = await SherpaAsrAPI.importModelArchive();
    return imported && {
      family: 'sherpa-onnx',
      modelId: imported.model_id,
      name: imported.model_id,
      path: imported.path,
    };
  },
  importDirectory: async (): Promise<ImportedOfflineModel | null> => {
    const imported = await SherpaAsrAPI.importModelDirectory();
    return imported && {
      family: 'sherpa-onnx',
      modelId: imported.model_id,
      name: imported.model_id,
      path: imported.path,
    };
  },
  importFile: async (): Promise<ImportedOfflineModel | null> => {
    const imported = await invoke<{ modelId: string; name: string; path: string } | null>(
      'whisper_import_model_file'
    );
    if (imported) notifyModelAssetsChanged('localWhisper');
    return imported && { family: 'whisper', ...imported };
  },
};
