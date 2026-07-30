import { getVersion } from '@tauri-apps/api/app';
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater';

export interface UpdateInfo {
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
  percentage: number | null;
}

class UpdateService {
  private pendingUpdate: Update | null = null;
  private checkPromise: Promise<UpdateInfo | null> | null = null;

  async checkForUpdates(): Promise<UpdateInfo | null> {
    if (this.checkPromise) return this.checkPromise;
    if (this.pendingUpdate) {
      return {
        currentVersion: this.pendingUpdate.currentVersion,
        version: this.pendingUpdate.version,
        date: this.pendingUpdate.date,
        body: this.pendingUpdate.body,
      };
    }

    this.checkPromise = (async () => {
      const [currentVersion, update] = await Promise.all([getVersion(), check({ timeout: 15_000 })]);
      if (!update) return null;

      this.pendingUpdate = update;
      return {
        currentVersion,
        version: update.version,
        date: update.date,
        body: update.body,
      };
    })();

    try {
      return await this.checkPromise;
    } finally {
      this.checkPromise = null;
    }
  }

  async downloadAndInstall(onProgress: (progress: UpdateProgress) => void): Promise<void> {
    const update = this.pendingUpdate;
    if (!update) throw new Error('No update is ready to download');

    let downloaded = 0;
    let total: number | null = null;
    const report = (event: DownloadEvent) => {
      if (event.event === 'Started') {
        total = event.data.contentLength ?? null;
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength;
      } else if (event.event === 'Finished' && total !== null) {
        downloaded = total;
      }
      onProgress({
        downloaded,
        total,
        percentage: total && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : null,
      });
    };

    await update.downloadAndInstall(report, { timeout: 10 * 60_000 });
  }
}

export const updateService = new UpdateService();
