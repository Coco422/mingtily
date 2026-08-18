"use client"

import { useEffect, useState } from "react"
import { Switch } from "./ui/switch"
import { Download, FolderOpen, Loader2, RefreshCw } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { useConfig, NotificationSettings } from "@/contexts/ConfigContext"
import { useTranslation } from "react-i18next"
import { setUiLocale } from "@/i18n"
import type { AppLocale } from "@/i18n/resources"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select"
import { toast } from "sonner"
import {
  AUTO_UPDATE_CHANGED_EVENT,
  AUTO_UPDATE_STORAGE_KEY,
  getCurrentUpdateState,
  requestUpdateCheck,
  requestUpdateInstall,
  requestUpdateRestart,
  UPDATE_STATE_EVENT,
  type UpdateState,
} from '@/components/UpdateCheckProvider'

interface DiagnosticExportResult {
  path: string;
  files_included: number;
}

export function PreferenceSettings() {
  const { t, i18n } = useTranslation('settings');
  const {
    notificationSettings,
    storageLocations,
    isLoadingPreferences,
    loadPreferences,
    updateNotificationSettings
  } = useConfig();

  const [notificationsEnabled, setNotificationsEnabled] = useState<boolean | null>(null);
  const [summaryNotificationsEnabled, setSummaryNotificationsEnabled] = useState(false);
  const [isInitialLoad, setIsInitialLoad] = useState(true);
  const [previousNotificationsEnabled, setPreviousNotificationsEnabled] = useState<boolean | null>(null);
  const [isExportingDiagnostics, setIsExportingDiagnostics] = useState(false);
  const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(false);
  const [updateState, setUpdateState] = useState<UpdateState>(getCurrentUpdateState);

  useEffect(() => {
    setAutoUpdateEnabled(localStorage.getItem(AUTO_UPDATE_STORAGE_KEY) === 'true');
    const handleState = (event: Event) => {
      setUpdateState((event as CustomEvent<UpdateState>).detail);
    };
    window.addEventListener(UPDATE_STATE_EVENT, handleState);
    return () => window.removeEventListener(UPDATE_STATE_EVENT, handleState);
  }, []);

  // Lazy load preferences on mount (only loads if not already cached)
  useEffect(() => {
    loadPreferences();
  }, [loadPreferences]);

  // Update notificationsEnabled when notificationSettings are loaded from global state
  useEffect(() => {
    if (notificationSettings) {
      // Notification enabled means both started and stopped notifications are enabled
      const enabled =
        notificationSettings.notification_preferences.show_recording_started &&
        notificationSettings.notification_preferences.show_recording_stopped;
      setNotificationsEnabled(enabled);
      setSummaryNotificationsEnabled(
        notificationSettings.notification_preferences.show_summary_completed &&
        notificationSettings.notification_preferences.show_summary_failed
      );
      if (isInitialLoad) {
        setPreviousNotificationsEnabled(enabled);
        setIsInitialLoad(false);
      }
    } else if (!isLoadingPreferences) {
      // If not loading and no settings, use default
      setNotificationsEnabled(true);
      if (isInitialLoad) {
        setPreviousNotificationsEnabled(true);
        setIsInitialLoad(false);
      }
    }
  }, [notificationSettings, isLoadingPreferences, isInitialLoad])

  useEffect(() => {
    // Skip update on initial load or if value hasn't actually changed
    if (isInitialLoad || notificationsEnabled === null || notificationsEnabled === previousNotificationsEnabled) return;
    if (!notificationSettings) return;

    const handleUpdateNotificationSettings = async () => {
      console.log("Updating notification settings to:", notificationsEnabled);

      try {
        // Update the notification preferences
        const updatedSettings: NotificationSettings = {
          ...notificationSettings,
          notification_preferences: {
            ...notificationSettings.notification_preferences,
            show_recording_started: notificationsEnabled,
            show_recording_stopped: notificationsEnabled,
          }
        };

        console.log("Calling updateNotificationSettings with:", updatedSettings);
        await updateNotificationSettings(updatedSettings);
        setPreviousNotificationsEnabled(notificationsEnabled);
        console.log("Successfully updated notification settings to:", notificationsEnabled);

      } catch (error) {
        console.error('Failed to update notification settings:', error);
      }
    };

    handleUpdateNotificationSettings();
  }, [notificationsEnabled, notificationSettings, isInitialLoad, previousNotificationsEnabled, updateNotificationSettings])

  const handleOpenFolder = async (folderType: 'database' | 'models' | 'recordings') => {
    try {
      switch (folderType) {
        case 'database':
          await invoke('open_database_folder');
          break;
        case 'models':
          await invoke('open_models_folder');
          break;
        case 'recordings':
          await invoke('open_recordings_folder');
          break;
      }
    } catch (error) {
      console.error(`Failed to open ${folderType} folder:`, error);
    }
  };

  const handleSummaryNotificationsChange = async (enabled: boolean) => {
    if (!notificationSettings) return;
    setSummaryNotificationsEnabled(enabled);
    try {
      await updateNotificationSettings({
        ...notificationSettings,
        notification_preferences: {
          ...notificationSettings.notification_preferences,
          show_summary_completed: enabled,
          show_summary_failed: enabled,
        },
      });
    } catch (error) {
      setSummaryNotificationsEnabled(!enabled);
      toast.error(t('general.notificationsSaveFailed'));
    }
  };

  const handleExportDiagnostics = async () => {
    setIsExportingDiagnostics(true);
    try {
      const result = await invoke<DiagnosticExportResult | null>('export_diagnostic_logs');
      if (result) {
        toast.success(t('general.diagnosticsExported'), {
          description: t('general.diagnosticsExportedDescription', { count: result.files_included }),
        });
      }
    } catch (error) {
      console.error('Failed to export diagnostics:', error);
      toast.error(t('general.diagnosticsExportFailed'));
    } finally {
      setIsExportingDiagnostics(false);
    }
  };

  const handleAutoUpdateChange = (enabled: boolean) => {
    setAutoUpdateEnabled(enabled);
    localStorage.setItem(AUTO_UPDATE_STORAGE_KEY, String(enabled));
    window.dispatchEvent(new CustomEvent(AUTO_UPDATE_CHANGED_EVENT));
  };

  const handleUpdateAction = () => {
    if (updateState.status === 'available') {
      requestUpdateInstall();
    } else if (updateState.status === 'ready') {
      requestUpdateRestart();
    } else {
      requestUpdateCheck(true);
    }
  };

  // Show loading only if we're actually loading and don't have cached data
  if (isLoadingPreferences && !notificationSettings && !storageLocations) {
    return <div className="max-w-2xl mx-auto p-6">{t('general.loading')}</div>
  }

  // Show loading if notificationsEnabled hasn't been determined yet
  if (notificationsEnabled === null && !isLoadingPreferences) {
    return <div className="max-w-2xl mx-auto p-6">{t('general.loading')}</div>
  }

  // Ensure we have a boolean value for the Switch component
  const notificationsEnabledValue = notificationsEnabled ?? false;

  return (
    <div className="space-y-6">
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">{t('general.appLanguage')}</h3>
            <p className="text-sm text-gray-600">{t('general.appLanguageDescription')}</p>
          </div>
          <Select
            value={(i18n.resolvedLanguage || 'en-US') as AppLocale}
            onValueChange={(locale) => void setUiLocale(locale as AppLocale)}
          >
            <SelectTrigger className="w-full sm:w-48" aria-label={t('general.appLanguage')}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="zh-CN">简体中文</SelectItem>
              <SelectItem value="en-US">English</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Notifications Section */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">{t('general.notifications')}</h3>
            <p className="text-sm text-gray-600">{t('general.notificationsDescription')}</p>
          </div>
          <Switch checked={notificationsEnabledValue} onCheckedChange={setNotificationsEnabled} />
        </div>
        <div className="mt-4 flex items-center justify-between border-t border-gray-100 pt-4">
          <div>
            <h4 className="text-sm font-medium text-gray-900">{t('general.summaryNotifications')}</h4>
            <p className="mt-1 text-xs text-gray-600">{t('general.summaryNotificationsDescription')}</p>
          </div>
          <Switch checked={summaryNotificationsEnabled} onCheckedChange={handleSummaryNotificationsChange} />
        </div>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="max-w-xl">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">{t('general.updates.title')}</h3>
            <p className="text-sm text-gray-600">{t('general.updates.description')}</p>
            <p className="mt-2 text-xs text-gray-500">{t('general.updates.privacy')}</p>
          </div>
          <Switch
            checked={updateState.status === 'disabled' ? false : autoUpdateEnabled}
            disabled={updateState.status === 'disabled'}
            onCheckedChange={handleAutoUpdateChange}
          />
        </div>
        <div className="mt-4 flex items-center justify-between gap-3 border-t pt-4">
          <p className="text-xs text-gray-500">
            {updateState.status === 'available' && t('general.updates.available', { version: updateState.version })}
            {updateState.status === 'downloading' && (
              updateState.percentage === null
                ? t('general.updates.downloading', { version: updateState.version })
                : t('general.updates.downloadingProgress', { version: updateState.version, progress: updateState.percentage })
            )}
            {updateState.status === 'ready' && t('general.updates.ready', { version: updateState.version })}
            {updateState.status === 'current' && t('general.updates.current')}
            {updateState.status === 'error' && t('general.updates.checkFailed')}
            {updateState.status === 'disabled' && t('general.updates.developmentDisabled')}
            {updateState.status === 'idle' && t('general.updates.idle')}
            {updateState.status === 'checking' && t('general.updates.checking')}
          </p>
          <button
            type="button"
            onClick={handleUpdateAction}
            disabled={updateState.status === 'disabled' || updateState.status === 'checking' || updateState.status === 'downloading'}
            className="inline-flex shrink-0 items-center justify-center gap-2 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {updateState.status === 'available' ? (
              <Download className="h-4 w-4" />
            ) : (
              <RefreshCw className={`h-4 w-4 ${updateState.status === 'checking' ? 'animate-spin' : ''}`} />
            )}
            {updateState.status === 'available'
              ? t('general.updates.download')
              : updateState.status === 'ready'
                ? t('general.updates.restartNow')
                : t('general.updates.checkNow')}
          </button>
        </div>
      </div>

      {/* Data Storage Locations Section */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">{t('general.storage')}</h3>
        <p className="text-sm text-gray-600 mb-6">
          {t('general.storageDescription')}
        </p>

        <div className="space-y-4">
          {/* Database Location */}
          {/* <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">Database</div>
            <div className="text-sm text-gray-600 mb-3 break-all font-mono text-xs">
              {storageLocations?.database || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('database')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Models Location */}
          {/* <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">Whisper Models</div>
            <div className="text-sm text-gray-600 mb-3 break-all font-mono text-xs">
              {storageLocations?.models || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('models')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Recordings Location */}
          <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">{t('general.recordingsFolder')}</div>
            <div className="text-sm text-gray-600 mb-3 break-all font-mono text-xs">
              {storageLocations?.recordings || t('general.loading')}
            </div>
            <button
              onClick={() => handleOpenFolder('recordings')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              {t('general.openFolder')}
            </button>
          </div>
        </div>

        <div className="mt-4 p-3 bg-blue-50 rounded-md">
          <p className="text-xs text-blue-800">
            <strong>{t('general.noteLabel')}</strong> {t('general.storageNote')}
          </p>
        </div>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="max-w-xl">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">{t('general.diagnostics')}</h3>
            <p className="text-sm text-gray-600">{t('general.diagnosticsDescription')}</p>
            <p className="mt-2 text-xs text-gray-500">{t('general.diagnosticsPrivacy')}</p>
          </div>
          <button
            type="button"
            onClick={() => void handleExportDiagnostics()}
            disabled={isExportingDiagnostics}
            className="inline-flex w-full items-center justify-center gap-2 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60 sm:w-auto"
          >
            {isExportingDiagnostics ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Download className="h-4 w-4" />
            )}
            {isExportingDiagnostics ? t('general.exportingDiagnostics') : t('general.exportDiagnostics')}
          </button>
        </div>
      </div>

    </div>
  )
}
