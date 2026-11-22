import { useEffect, useRef, useState } from 'react';
import type { Api, ResortPlanDto } from '../../api';
import { formatNumber, t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { Modal } from '../Modal';
import './organized.css';

/**
 * "Sắp xếp lại" (class đổi sau khi file đã xếp): xem trước từ → đến, xác nhận, chạy, rồi thông báo có Hoàn tác (giữ batchId).
 * Không bao giờ tự chạy; font đang bật lâu dài do backend bỏ qua.
 */
export function ResortDialog({ api, onClose }: { api: Api; onClose(): void }): React.JSX.Element {
  const [plan, setPlan] = useState<ResortPlanDto | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; api.organizedResortPreview().then(p => { if (alive.current) setPlan(p); }, e => { if (alive.current) setErr((e as Error).message); }); return () => { alive.current = false; }; }, [api]);
  const apply = async () => {
    if (!plan) return;
    setBusy(true);
    // Tiến độ và nút Huỷ nằm ở khay Tác vụ: đóng hộp ngay, kết quả báo bằng thông báo khi việc kết thúc (xong hoặc huỷ).
    onClose();
    try {
      const r = await api.organizedResortApply();
      useUiStore.getState().bumpLibrary(false);
      const text = r.cancelled ? (r.moved > 0 ? t('org.resort.cancelled', { n: formatNumber(r.moved) }) : t('org.resort.cancelledNone'))
        : r.failed > 0 ? t('org.resort.doneFail', { n: formatNumber(r.moved), k: formatNumber(r.failed) }) : t('org.resort.done', { n: formatNumber(r.moved) });
      useNoticeStore.getState().push({
        kind: r.failed > 0 ? 'error' : 'info', text,
        // Huỷ giữa chừng vẫn Hoàn tác được phần đã dời (cùng batchId); huỷ khi chưa dời gì thì không có gì để hoàn tác.
        ...(r.moved > 0 ? { action: { label: t('dnd.undo'), run: () => undo(r.batchId) } } : {}),
      });
    } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: (e as Error)?.message ?? String(e) }); }
  };
  const undo = (batchId: number) => {
    api.organizedResortUndo(batchId).then(u => {
      useUiStore.getState().bumpLibrary(false);
      useNoticeStore.getState().push({ kind: 'info', text: u.cancelled ? t('org.resort.undoCancelled', { n: formatNumber(u.moved) }) : t('org.resort.undone', { n: formatNumber(u.moved) }),
        ...(u.cancelled ? { action: { label: t('dnd.undo'), run: () => undo(batchId) } } : {}) });
    }, e => useNoticeStore.getState().push({ kind: 'error', text: t('dnd.undoFail', { msg: (e as Error)?.message ?? String(e) }) }));
  };
  return (
    <Modal title={t('org.resort.title')} onClose={onClose} className="fa-org-dialog">
      {!plan && !err && <p role="status">{t('org.resort.loading')}</p>}
      {plan && plan.total === 0 && <p data-testid="org-resort-none">{t('org.resort.none')}</p>}
      {plan && plan.total > 0 && (
        <>
          <p>{t('org.resort.intro', { n: formatNumber(plan.total) })}</p>
          <ul className="fa-org-moves" data-testid="org-moves">{plan.moves.map((m, i) => <li key={i}><b>{m.from}</b><span>→</span><b>{m.to}</b></li>)}</ul>
          {plan.total > plan.moves.length && <p className="fa-org-note">{t('org.resort.more', { n: formatNumber(plan.total - plan.moves.length) })}</p>}
          <p className="fa-org-note">{t('org.resort.activeNote')}</p>
        </>
      )}
      {err && <p className="fa-org-err" role="alert">{err}</p>}
      <div className="fa-modal-actions">
        <button type="button" onClick={onClose}>{t('org.cancel')}</button>
        <button type="button" className="primary" data-testid="org-resort-apply" disabled={!plan || plan.total === 0 || busy} onClick={() => void apply()}>{plan ? t('org.resort.apply', { n: formatNumber(plan.total) }) : t('org.resort.applyBare')}</button>
      </div>
    </Modal>
  );
}
