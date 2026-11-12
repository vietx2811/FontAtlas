import React from 'react';
import type { Api } from '../../api';
import type { SourceDto } from '../../api/types';
import { Tooltip } from '../ui/Tooltip';
import { cancelSourceDownload, downloadSourceToMac, useCloudDownloadStore } from '../../store/cloudDownload';
import { t } from '../../i18n';

/** Nút "Tải về máy" cạnh dòng "N chưa tải về" (Sidebar, Cài đặt): đang tải thì hiện tiến độ và nút Huỷ. */
export function CloudDownloadControl({ api, source, notDownloaded, className }: { api: Api; source: Pick<SourceDto, 'id' | 'name' | 'available'>; notDownloaded: number; className?: string }): React.JSX.Element | null {
  const run = useCloudDownloadStore(s => s.runs[source.id]);
  if (run) {
    return (
      <span className={className} data-testid="cloud-dl-running" role="status">
        {run.total > 0 ? t('cdl.running', { done: run.done, total: run.total }) : t('cdl.starting')}
        <button type="button" className="fa-link" data-testid="cloud-dl-cancel" disabled={run.cancelling} onClick={() => void cancelSourceDownload(api, source.id)}>
          {run.cancelling ? t('cdl.cancelling') : t('cdl.cancel')}
        </button>
      </span>
    );
  }
  if (notDownloaded <= 0 || !source.available) return null;
  return (
    <Tooltip label={t('cdl.tip')}>
      <button type="button" className={`fa-link${className ? ` ${className}` : ''}`} data-testid="cloud-dl-start" onClick={() => void downloadSourceToMac(api, source)}>
        {t('cdl.btn')}
      </button>
    </Tooltip>
  );
}
