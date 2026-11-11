import { useEffect, useMemo, useRef, useState } from 'react';
import type { Api, FaceQuery, PlanItemDto, ResolvePlanDto, ResolveResultDto, ResolveScopeDto } from '../../api';
import { t } from '../../i18n';
import { Modal } from '../Modal';
import { applyPlan } from './applyPlan';
import { itemId, memberName, splitForTrash, trashState, viewOf } from './dupPlan';
import { blockedText, formatBytes, reasonText } from './dupText';
import { splitPath, tailDepth } from './duplicatesFormat';

/**
 * Màn xem trước trước khi áp dụng: từng nhóm giữ/ẩn bản nào, tổng số, cách xử lý bản thừa (Ẩn là mặc định; Thùng rác là lựa chọn phụ
 * có liệt kê TỪNG tệp và tick riêng cho tệp ở nơi cần xác nhận). Lập lại kế hoạch mỗi lần mở; kế hoạch cũ → `plan_stale` → lập lại.
 */
export function ResolveDialog({ api, scope, q, choices, onClose, onDone }: {
  api: Api; scope: ResolveScopeDto; q: FaceQuery; choices: Record<string, string>; onClose(): void; onDone(r: ResolveResultDto, action: 'hide' | 'trash'): void;
}): React.JSX.Element {
  const [plan, setPlan] = useState<ResolvePlanDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [action, setAction] = useState<'hide' | 'trash'>('hide');
  const [ticked, setTicked] = useState<ReadonlySet<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [stale, setStale] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const seq = useRef(0);
  const scopeKey = JSON.stringify(scope);

  useEffect(() => {
    const mine = ++seq.current; setPlan(null); setError(null);
    api.resolvePreview(scope, q).then(p => { if (mine === seq.current) setPlan(p); }, e => { if (mine === seq.current) setError(e instanceof Error ? e.message : String(e)); });
    return () => { seq.current++; };
  }, [api, scopeKey, attempt]); // eslint-disable-line react-hooks/exhaustive-deps

  const allSafe = scope.type === 'all_safe';
  const items: PlanItemDto[] = plan?.items ?? [];
  const views = useMemo(() => items.map(it => ({ it, v: viewOf(it, choices[itemId(it)]) })), [items, choices]);
  const losers = views.flatMap(x => x.v.losers);
  const split = splitForTrash(losers, ticked);
  const trashBytes = split.trash.reduce((n, l) => n + l.fileSize, 0);
  const changed = Object.keys(choices).length > 0 && views.some(x => x.v.chosen);
  const canGo = !!plan && items.length > 0 && !busy && (action === 'hide' || split.trash.length > 0);

  const go = async () => {
    if (!canGo) return;
    setBusy(true); setStale(false);
    try {
      const r = await applyPlan({ api, items, choices, action, ticked, allSafe: allSafe && !changed });
      if (r.done.length === 0 && r.failed.some(f => f.code === 'plan_stale')) { setStale(true); setAttempt(n => n + 1); setBusy(false); return; }
      onDone(r, action);
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); setBusy(false); }
  };

  return (
    <Modal title={allSafe ? t('dup.pv.title.all', { n: plan?.items.length ?? 0 }) : t('dup.pv.title.one')} onClose={() => { if (!busy) onClose(); }} className="dup-pv">
      {error && <p role="alert" className="dup-pv-err">{t('dup.pv.error', { msg: error })} <button type="button" className="dup-linkbtn" onClick={() => setAttempt(n => n + 1)}>{t('dup.retry')}</button></p>}
      {!error && !plan && <p role="status">{t('dup.pv.loading')}</p>}
      {stale && <p role="alert" className="dup-pv-err">{t('dup.pv.stale')}</p>}
      {plan && items.length === 0 && <p>{t('dup.pv.empty')}</p>}
      {plan && items.length > 0 && <>
        <p className="dup-pv-sum" data-testid="pv-summary">{t('dup.pv.summary', { keep: items.length, hide: losers.length })}{allSafe && plan.needsReview > 0 && ` ${t('dup.pv.needsReview', { n: plan.needsReview })}`}</p>
        <ul className="dup-pv-list" aria-label={t('dup.pv.list')}>
          {views.map(({ it, v }) => { const dp = tailDepth(it.memberPaths); return (
            <li key={itemId(it)} data-testid="pv-item">
              <b>{memberName(v.keeper)}</b>
              <span className="dup-pv-row keep"><em>{t('dup.pv.keepRow')}</em> {v.keeper.sourceName} · {splitPath(v.keeper.path, dp).tail}</span>
              {v.losers.map(l => <span key={l.path} className="dup-pv-row hide"><em>{t('dup.pv.hideRow')}</em> {l.sourceName} · {splitPath(l.path, dp).tail}</span>)}
              <small>{reasonText(v.reason)}</small>
            </li>); })}
        </ul>
        <fieldset className="dup-pv-act">
          <legend>{t('dup.pv.action')}</legend>
          <label className={action === 'hide' ? 'on' : ''}><input type="radio" name="dup-act" checked={action === 'hide'} onChange={() => setAction('hide')} />
            <span><b>{t('dup.pv.hide')}</b> <i className="dup-tag">{t('dup.pv.hide.tag')}</i><small>{t('dup.pv.hide.sub')}</small></span></label>
          <label className={action === 'trash' ? 'on' : ''}><input type="radio" name="dup-act" checked={action === 'trash'} onChange={() => setAction('trash')} />
            <span><b>{t('dup.pv.trash')}</b><small>{t('dup.pv.trash.sub', { size: formatBytes(plan.reclaimableBytes) })}</small></span></label>
        </fieldset>
        {action === 'trash' && (
          <div className="dup-pv-trash" data-testid="pv-trash">
            <p>{t('dup.pv.trash.note')}</p>
            {split.trash.length === 0 && <p>{t('dup.pv.trash.none')}</p>}
            <ul aria-label={t('dup.pv.trash.files')}>
              {losers.map(l => { const s = trashState(l); return (
                <li key={l.path} data-state={s}>
                  {s === 'explicit'
                    ? <label><input type="checkbox" checked={ticked.has(l.path)} onChange={e => setTicked(prev => { const n = new Set(prev); if (e.target.checked) n.add(l.path); else n.delete(l.path); return n; })} />
                      <span><b>{splitPath(l.path, 2).tail}</b> · {formatBytes(l.fileSize)}<small>{splitPath(l.path, 2).head}</small><small className="warn">{t('dup.pv.trash.explicit')}</small></span></label>
                    : <span><b>{splitPath(l.path, 2).tail}</b> · {formatBytes(l.fileSize)}<small>{splitPath(l.path, 2).head}</small>{s === 'blocked' && <small className="warn">{t('dup.pv.trash.blocked', { why: blockedText(l.trash.blocked) })}</small>}</span>}
                </li>); })}
            </ul>
            {split.trash.length > 0 && <p className="dup-pv-total" data-testid="pv-total">{t('dup.pv.trash.total', { n: split.trash.length, size: formatBytes(trashBytes) })}</p>}
            {split.hideOnly.length > 0 && <p>{t('dup.pv.trash.skipped', { n: split.hideOnly.length })}</p>}
          </div>
        )}
      </>}
      <div className="fa-modal-actions">
        <button type="button" onClick={onClose} aria-disabled={busy}>{t('dup.pv.cancel')}</button>
        <button type="button" className="primary" aria-disabled={!canGo} onClick={() => void go()} data-testid="pv-confirm">
          {busy ? t('dup.pv.working') : action === 'hide' ? t('dup.pv.confirm.hide', { n: losers.length }) : t('dup.pv.confirm.trash', { n: split.trash.length })}
        </button>
      </div>
    </Modal>
  );
}

