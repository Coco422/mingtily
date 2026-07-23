'use client';

import { useTranslation } from 'react-i18next';

export function NoteNotFound() {
  const { t } = useTranslation('meeting');

  return <div className="p-8">{t('noteNotFound')}</div>;
}
