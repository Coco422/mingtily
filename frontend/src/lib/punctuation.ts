import { invoke } from '@tauri-apps/api/core';

export type PunctuationModelState = 'available' | 'missing' | 'corrupt';

export interface PunctuationModelStatus {
  id: string;
  name: string;
  status: PunctuationModelState;
  download_size: number;
  installed_size: number;
  languages: string[];
  license: string;
  path: string;
  error?: string | null;
}

export interface PunctuationDownloadProgress {
  model_id: string;
  progress: number;
  downloaded_bytes: number;
  total_bytes: number;
  downloaded_mb: number;
  total_mb: number;
  status: string;
}

export const PunctuationAPI = {
  getStatus: () => invoke<PunctuationModelStatus>('punctuation_get_status'),
  downloadModel: () => invoke<void>('punctuation_download_model'),
  deleteModel: () => invoke<void>('punctuation_delete_model'),
};
