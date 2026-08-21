'use client';

import { useState } from 'react';
import { File, FileArchive, FolderOpen, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { offlineModelImportService } from '@/services/offlineModelImportService';

export function OfflineModelImport() {
  const { t } = useTranslation('models');
  const [importing, setImporting] = useState<'file' | 'archive' | 'directory' | null>(null);

  const importOffline = async (kind: 'file' | 'archive' | 'directory') => {
    setImporting(kind);
    try {
      const imported = kind === 'file'
        ? await offlineModelImportService.importFile()
        : kind === 'archive'
          ? await offlineModelImportService.importArchive()
          : await offlineModelImportService.importDirectory();
      if (!imported) return;
      toast.success(t('offlineImport.success', { model: imported.name }));
    } catch (error) {
      toast.error(t('offlineImport.failed'), { description: String(error) });
    } finally {
      setImporting(null);
    }
  };

  return (
    <div>
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          size="sm"
          disabled={importing !== null}
          onClick={() => void importOffline('file')}
        >
          {importing === 'file' ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <File className="mr-2 h-4 w-4" />
          )}
          {t('offlineImport.file')}
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled={importing !== null}
          onClick={() => void importOffline('archive')}
        >
          {importing === 'archive' ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <FileArchive className="mr-2 h-4 w-4" />
          )}
          {t('offlineImport.archive')}
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled={importing !== null}
          onClick={() => void importOffline('directory')}
        >
          {importing === 'directory' ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <FolderOpen className="mr-2 h-4 w-4" />
          )}
          {t('offlineImport.directory')}
        </Button>
      </div>
      <p className="mt-2 text-xs leading-5 text-muted-foreground">
        {t('offlineImport.hint')}
      </p>
    </div>
  );
}
