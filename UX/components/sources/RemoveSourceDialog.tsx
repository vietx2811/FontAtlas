import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api, SourceDto } from '../../api';
import type { RemoveSourceReportDto } from '../../bindings';
import { t } from '../../i18n';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { useScanStore } from '../../store/scan';

type Props = { api: Api; source: SourceDto; onClose(): void; onRemoved(report: RemoveSourceReportDto): void };

const msg = (e: unknown): string => (e instanceof Error ? e.message : String(e));

/** Gỡ nguồn (spec C6 / R7): số liệu THẬT từ `removeSourcePreview` trước khi xác nhận. Không xoá file font; Dự án, Thẻ, Yêu thích được giữ. */
export function RemoveSourceDialog({ api, source, onClose, onRemoved }: Props): React.JSX.Element {
  const [report, setReport] = useState<RemoveSourceReportDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const scanning = useScanStore(s => s.scanning);
  const running = useRef(false);
  const seq = useRef(0);
  const alive = useRef(true);
  useEffect(() => () => { alive.current = false; }, []);

  const load = useCallback(() => {
    const my = ++seq.current;
    setReport(null); setError(null);
    api.removeSourcePreview(source.id).then(r => { if (alive.current && my === seq.current) setReport(r); }, e => { if (alive.current && my === seq.current) setError(t('rmsrc.previewFail', { msg: msg(e) })); });
  }, [api, source.id]);
  useEffect(() => { load(); }, [load]);

  const go = async () => {
    if (running.current || !report) return;
    running.current = true; setBusy(true); setError(null);
    try {
      // Số liệu đã xem có thể cũ (đang quét, hoặc thư viện đổi từ lúc mở hộp): lấy lại; khác → cho xem số mới rồi bấm lại, không gỡ theo số cũ.
      const fresh = await api.removeSourcePreview(source.id);
      if (JSON.stringify(fresh) !== JSON.stringify(report)) { if (alive.current) { setReport(fresh); setError(t('rmsrc.changed')); } return; }
      const r = await api.removeSource(source.id); onRemoved(r);
    }
    catch (e) { if (alive.current) setError(t('rmsrc.fail', { msg: msg(e) })); }
    finally { running.current = false; if (alive.current) setBusy(false); }
  };

  return (
    <Modal title={t('rmsrc.title', { name: source.name })} onClose={() => { if (!running.current) onClose(); }}>
      <div data-testid="rmsrc">
        <p>{t('rmsrc.keepFiles')}</p>
        {!report && !error && <p role="status" data-testid="rmsrc-loading">{t('rmsrc.loading')}</p>}
        {scanning && <p className="fa-set-note" data-testid="rmsrc-scanning"><NoteMark inline kind="warn" text={t('rmsrc.scanning.tip')} /> {t('rmsrc.scanning')}</p>}
        {report && (
          <ul className="fa-rmsrc-list" data-testid="rmsrc-facts">
            <li data-fact="locations">{t('rmsrc.locations', { n: report.removedLocations })}</li>
            {report.facesLeftWithoutLocation > 0 && <li data-fact="orphans">{t('rmsrc.orphans', { n: report.facesLeftWithoutLocation })}</li>}
            {report.casesAffected > 0 && <li data-fact="cases">{t('rmsrc.cases', { n: report.casesAffected })}</li>}
            {report.collectionsAffected > 0 && <li data-fact="collections">{t('rmsrc.collections', { n: report.collectionsAffected })}</li>}
            {report.tagsAffected > 0 && <li data-fact="tags">{t('rmsrc.tags', { n: report.tagsAffected })}</li>}
            {report.activeFonts > 0 && <li data-fact="active"><NoteMark inline kind="warn" testId="rmsrc-active-note" text={t('rmsrc.active.tip')} /> {t('rmsrc.active', { n: report.activeFonts })}</li>}
          </ul>
        )}
        {report && report.facesLeftWithoutLocation > 0 && <p className="fa-set-note" data-testid="rmsrc-readd">{t('rmsrc.readd')}</p>}
        {error && (
          <p role="alert" data-testid="rmsrc-error">{error}{!report && <> <button type="button" data-testid="rmsrc-retry" onClick={load}>{t('rmsrc.retry')}</button></>}</p>
        )}
        <div className="fa-modal-actions">
          <button type="button" data-testid="rmsrc-cancel" aria-disabled={busy || undefined} onClick={() => { if (!busy) onClose(); }}>{t('rmsrc.cancel')}</button>
          <button type="button" className="primary" data-testid="rmsrc-confirm" aria-disabled={!report || busy || undefined} onClick={() => void go()}>{t('rmsrc.ok')}</button>
        </div>
      </div>
    </Modal>
  );
}
