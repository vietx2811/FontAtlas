import React from 'react';
import type { Api } from '../../api';
import type { SourceDto } from '../../api/types';
import { Tooltip } from '../ui/Tooltip';
import { cancelSourceDownload, downloadSourceToMac, useCloudDownloadStore } from '../../store/cloudDownload';
import { t } from '../../i18n';
import { Icon } from './icons';

/** Nút "Tải về máy" cạnh dòng "N chưa tải về" (Sidebar, Cài đặt): đang tải thì hiện tiến độ và nút Huỷ. */
export function CloudDownloadControl({ api, source, notDownloaded, className, compact = false }: { api: Api; source: Pick<SourceDto, 'id' | 'name' | 'available'>; notDownloaded: number; className?: string; /** Chỉ icon + số (Sidebar): chữ giải thích nằm trong Tooltip/aria-label. */ compact?: boolean }): React.JSX.Element | null {
  const run = useCloudDownloadStore(s => s.runs[source.id]);
  if (run && compact) {
    const label = run.total > 0 ? t('cdl.running', { done: run.done, total: run.total }) : t('cdl.starting');
    const cancel = run.cancelling ? t('cdl.cancelling') : t('cdl.cancel');
    return (
      <span className={className}>
        <span data-testid="cloud-dl-running" role="status" aria-label={label}><Tooltip label={label}><span className="fa-cloud-run" tabIndex={0}><Icon name="download" size={12} />{run.total > 0 ? `${run.done}/${run.total}` : '…'}</span></Tooltip></span>
        <Tooltip label={cancel}><button type="button" className="fa-link fa-cloud-x" data-testid="cloud-dl-cancel" aria-label={cancel} disabled={run.cancelling} onClick={() => void cancelSourceDownload(api, source.id)}><Icon name="x" size={11} /></button></Tooltip>
      </span>
    );
  }
  if (run) {
    return (
      <span className={className}>
        <span data-testid="cloud-dl-running" role="status">{run.total > 0 ? t('cdl.running', { done: run.done, total: run.total }) : t('cdl.starting')}</span>{' '}
        <button type="button" className="fa-link" data-testid="cloud-dl-cancel" disabled={run.cancelling} onClick={() => void cancelSourceDownload(api, source.id)}>
          {run.cancelling ? t('cdl.cancelling') : t('cdl.cancel')}
        </button>
      </span>
    );
  }
  if (notDownloaded <= 0 || !source.available) return null;
  if (compact) {
    return (
      <Tooltip label={t('cdl.tip')}>
        <button type="button" className={`fa-link fa-cloud-btn${className ? ` ${className}` : ''}`} data-testid="cloud-dl-start" aria-label={t('cdl.btn')} onClick={() => void downloadSourceToMac(api, source)}><Icon name="download" size={13} /></button>
      </Tooltip>
    );
  }
  return (
    <Tooltip label={t('cdl.tip')}>
      <button type="button" className={`fa-link${className ? ` ${className}` : ''}`} data-testid="cloud-dl-start" onClick={() => void downloadSourceToMac(api, source)}>
        {t('cdl.btn')}
      </button>
    </Tooltip>
  );
}
