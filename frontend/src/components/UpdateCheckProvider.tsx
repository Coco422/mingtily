'use client';

import { useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { updateService, type UpdateInfo } from '@/services/updateService';

export const AUTO_UPDATE_STORAGE_KEY = 'mingtily:auto-update-enabled';
export const UPDATE_CHECK_REQUEST_EVENT = 'mingtily:check-for-updates';
export const UPDATE_INSTALL_REQUEST_EVENT = 'mingtily:install-update';
export const UPDATE_RESTART_REQUEST_EVENT = 'mingtily:restart-after-update';
export const AUTO_UPDATE_CHANGED_EVENT = 'mingtily:auto-update-changed';
export const UPDATE_STATE_EVENT = 'mingtily:update-state';

export type UpdateState =
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'current' }
  | { status: 'available'; version: string }
  | { status: 'downloading'; version: string; percentage: number | null }
  | { status: 'ready'; version: string }
  | { status: 'error'; message: string };

let currentUpdateState: UpdateState = { status: 'idle' };

export function getCurrentUpdateState() {
  return currentUpdateState;
}

function emitState(state: UpdateState) {
  currentUpdateState = state;
  window.dispatchEvent(new CustomEvent<UpdateState>(UPDATE_STATE_EVENT, { detail: state }));
}

function autoUpdateEnabled() {
  return localStorage.getItem(AUTO_UPDATE_STORAGE_KEY) === 'true';
}

export function requestUpdateCheck(manual = true) {
  window.dispatchEvent(
    new CustomEvent<{ manual: boolean }>(UPDATE_CHECK_REQUEST_EVENT, { detail: { manual } })
  );
}

export function requestUpdateInstall() {
  window.dispatchEvent(new CustomEvent(UPDATE_INSTALL_REQUEST_EVENT));
}

export function requestUpdateRestart() {
  window.dispatchEvent(new CustomEvent(UPDATE_RESTART_REQUEST_EVENT));
}

export function UpdateCheckProvider() {
  const { t } = useTranslation('settings');
  const checkingRef = useRef(false);
  const installingRef = useRef(false);
  const installedVersionRef = useRef<string | null>(null);
  const availableInfoRef = useRef<UpdateInfo | null>(null);

  const restart = useCallback(async () => {
    try {
      if (await invoke<boolean>('is_recording')) {
        toast.warning(t('general.updates.restartBlocked'));
        return;
      }
      await relaunch();
    } catch (error) {
      toast.error(t('general.updates.restartFailed'), { description: String(error) });
    }
  }, [t]);

  const install = useCallback(async (info: UpdateInfo) => {
    if (installedVersionRef.current === info.version) {
      await restart();
      return;
    }
    if (installingRef.current) return;
    installingRef.current = true;
    const toastId = toast.loading(t('general.updates.downloading', { version: info.version }));
    try {
      await updateService.downloadAndInstall(({ percentage }) => {
        emitState({ status: 'downloading', version: info.version, percentage });
        toast.loading(
          percentage === null
            ? t('general.updates.downloading', { version: info.version })
            : t('general.updates.downloadingProgress', { version: info.version, progress: percentage }),
          { id: toastId }
        );
      });
      installedVersionRef.current = info.version;
      availableInfoRef.current = null;
      emitState({ status: 'ready', version: info.version });
      toast.success(t('general.updates.ready', { version: info.version }), {
        id: toastId,
        duration: Infinity,
        action: { label: t('general.updates.restartNow'), onClick: () => void restart() },
      });
    } catch (error) {
      emitState({ status: 'error', message: String(error) });
      toast.error(t('general.updates.installFailed'), { id: toastId, description: String(error) });
    } finally {
      installingRef.current = false;
    }
  }, [restart, t]);

  const checkForUpdates = useCallback(async (manual: boolean) => {
    if (checkingRef.current) return;
    checkingRef.current = true;
    emitState({ status: 'checking' });
    try {
      const info = await updateService.checkForUpdates();
      if (!info) {
        availableInfoRef.current = null;
        emitState({ status: 'current' });
        if (manual) toast.success(t('general.updates.current'));
        return;
      }

      availableInfoRef.current = info;
      emitState({ status: 'available', version: info.version });
      toast.info(t('general.updates.available', { version: info.version }), {
        duration: Infinity,
        description: info.body || undefined,
        action: {
          label: t('general.updates.download'),
          onClick: () => void install(info),
        },
      });
    } catch (error) {
      emitState({ status: 'error', message: String(error) });
      console.error('[Updater] Update check failed:', error);
      if (manual) {
        toast.error(t('general.updates.checkFailed'), { description: String(error) });
      }
    } finally {
      checkingRef.current = false;
    }
  }, [install, t]);

  useEffect(() => {
    const handleCheck = (event: Event) => {
      const detail = (event as CustomEvent<{ manual?: boolean }>).detail;
      void checkForUpdates(detail?.manual ?? true);
    };
    const handlePreferenceChange = () => {
      if (autoUpdateEnabled()) void checkForUpdates(true);
    };
    const handleInstall = () => {
      if (availableInfoRef.current) void install(availableInfoRef.current);
    };
    const handleRestart = () => void restart();
    window.addEventListener(UPDATE_CHECK_REQUEST_EVENT, handleCheck);
    window.addEventListener(AUTO_UPDATE_CHANGED_EVENT, handlePreferenceChange);
    window.addEventListener(UPDATE_INSTALL_REQUEST_EVENT, handleInstall);
    window.addEventListener(UPDATE_RESTART_REQUEST_EVENT, handleRestart);

    const timer = autoUpdateEnabled()
      ? window.setTimeout(() => void checkForUpdates(false), 2_000)
      : null;
    return () => {
      if (timer !== null) window.clearTimeout(timer);
      window.removeEventListener(UPDATE_CHECK_REQUEST_EVENT, handleCheck);
      window.removeEventListener(AUTO_UPDATE_CHANGED_EVENT, handlePreferenceChange);
      window.removeEventListener(UPDATE_INSTALL_REQUEST_EVENT, handleInstall);
      window.removeEventListener(UPDATE_RESTART_REQUEST_EVENT, handleRestart);
    };
  }, [checkForUpdates, install, restart]);

  return null;
}
