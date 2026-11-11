import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Api, ResolvePlanDto, ResolveResultDto, ResolveScopeDto } from '../../api';
import { t } from '../../i18n';
import { dropTags, toMetadataQuery, useUiStore } from '../../store/ui';
import { useScanStore } from '../../store/scan';
import { useNoticeStore } from '../../store/notices';
import { ZeroState } from '../zero/ZeroState';
import { Tooltip } from '../ui/Tooltip';
import { itemId, memberName, viewOf } from './dupPlan';
import { GroupCard, kindGloss, kindLabel } from './GroupCard';
import { IgnoredTab } from './IgnoredTab';
import { ResolveDialog } from './ResolveDialog';
import { ResolvedTab } from './ResolvedTab';
import { ResultDialog } from './ResultDialog';
import { undoWithFeedback } from './undoFlow';
import './duplicates.css';

const KINDS = ['same_file', 'versions', 'formats'] as const;
type Tab = 'todo' | 'done' | 'ignored';
const TABS: Tab[] = ['todo', 'done', 'ignored'];
const tabName = (x: Tab) => t(`dup.tab.${x}` as 'dup.tab.todo');

/**
 * Trang Trùng lặp: giải quyết nhóm font trùng. Mặc định Ẩn (không đụng tệp); Thùng rác là lựa chọn phụ có xác nhận từng tệp.
 * Tab "Cần xử lý" (máy chắc → "Giải quyết N nhóm an toàn"; máy chưa chắc → "Cần bạn xem"), "Đã giải quyết" (hoàn tác), "Đã bỏ qua".
 */
export function DuplicatesPage({ api }: { api: Api }): React.JSX.Element {
  const ui = useUiStore();
  const version = useScanStore(s => s.version);
  const q = useMemo(() => toMetadataQuery({ view: 'all', query: ui.query, viOnly: ui.viOnly, filters: dropTags(ui.filters) }), [ui.query, ui.viOnly, ui.filters]);
  const qKey = JSON.stringify(q);
  const [tab, setTab] = useState<Tab>('todo');
  const [plan, setPlan] = useState<ResolvePlanDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [choices, setChoices] = useState<Record<string, string>>({});
  const [dialog, setDialog] = useState<ResolveScopeDto | null>(null);
  const [result, setResult] = useState<ResolveResultDto | null>(null);
  const [busy, setBusy] = useState(false);
  const seq = useRef(0);

  const load = useCallback(() => {
    const mine = ++seq.current; setError(null);
    api.resolvePreview({ type: 'all' }, q).then(p => {
      if (mine !== seq.current) return;
      setPlan(p);
      // Lựa chọn "Đổi bản giữ" chỉ còn hiệu lực khi nhóm vẫn có đúng bản đó.
      setChoices(prev => { const next: Record<string, string> = {}; for (const it of p.items) { const c = prev[itemId(it)]; if (c && it.memberPaths.includes(c)) next[itemId(it)] = c; } return next; });
    }, e => { if (mine === seq.current) { setPlan(null); setError(e instanceof Error ? e.message : String(e)); } });
  }, [api, q]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { load(); return () => { seq.current++; }; }, [load, qKey, version, ui.libraryRev, attempt]); // eslint-disable-line react-hooks/exhaustive-deps

  const items = plan?.items ?? [];
  const safe = items.filter(i => i.autoSafe), review = items.filter(i => !i.autoSafe);

  const onDone = (r: ResolveResultDto, action: 'hide' | 'trash') => {
    setDialog(null);
    if (r.done.length > 0) {
      useUiStore.getState().bumpLibrary(true);
      const n = r.done.length;
      useNoticeStore.getState().push({
        kind: 'info', text: action === 'trash' && r.done.every(d => d.action === 'trash') ? t('dup.res.trashed', { n }) : t('dup.res.hidden', { n }),
        action: { label: t('dup.res.undo'), run: () => { void undoWithFeedback(api, r.done.map(d => d.id)); } },
      });
    }
    if (r.failed.length > 0 || r.done.length === 0) setResult(r);
  };

  const ignore = async (it: (typeof items)[number]) => {
    if (busy) return; setBusy(true);
    const name = memberName(viewOf(it, choices[itemId(it)]).keeper);
    try {
      await api.duplicateIgnore(it.kind, it.key);
      useUiStore.getState().bumpLibrary();
      useNoticeStore.getState().push({ kind: 'info', text: t('dup.ign.done', { name }), action: { label: t('dup.res.undo'), run: () => { void api.duplicateUnignore(it.kind, it.key).then(() => useUiStore.getState().bumpLibrary()); } } });
    } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('dup.ign.fail', { msg: e instanceof Error ? e.message : String(e) }) }); }
    finally { setBusy(false); }
  };

  const card = (it: (typeof items)[number]) => (
    <GroupCard key={itemId(it)} api={api} item={it} choice={choices[itemId(it)]} busy={busy}
      onSwap={p => setChoices(c => ({ ...c, [itemId(it)]: p }))}
      onResolve={() => setDialog({ type: 'group', kind: it.kind, key: it.key })}
      onIgnore={() => void ignore(it)} />
  );
  const onTabKey = (e: React.KeyboardEvent) => {
    const i = TABS.indexOf(tab); const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!d) return; e.preventDefault(); const nx = TABS[(i + d + TABS.length) % TABS.length]; setTab(nx);
    requestAnimationFrame(() => document.getElementById(`dup-tab-${nx}`)?.focus());
  };

  return (
    <section className="dup" aria-label={t('dup.title')}>
      <header className="dup-h">
        <h1>{t('dup.title')}</h1>
        <div role="tablist" aria-label={t('dup.tabs')} className="dup-tabs" onKeyDown={onTabKey}>
          {TABS.map(x => <button key={x} id={`dup-tab-${x}`} role="tab" type="button" aria-selected={tab === x} aria-controls="dup-panel" tabIndex={tab === x ? 0 : -1} onClick={() => setTab(x)}>{tabName(x)}{x === 'todo' && plan && items.length > 0 && <span className="dup-total">{items.length}</span>}</button>)}
        </div>
      </header>
      <div id="dup-panel" role="tabpanel" aria-labelledby={`dup-tab-${tab}`} className="dup-body">
        {tab === 'done' && <ResolvedTab api={api} />}
        {tab === 'ignored' && <IgnoredTab api={api} />}
        {tab === 'todo' && <>
          {error && <div role="alert" className="dup-state warn">{t('dup.error', { msg: error })} <button type="button" onClick={() => setAttempt(n => n + 1)}>{t('dup.retry')}</button></div>}
          {!error && !plan && <div role="status" className="dup-state">{t('dup.loading')}</div>}
          {plan && items.length === 0 && <ZeroState glyph="=" text={t('dup.empty')} />}
          {plan && items.length > 0 && (
            <div className="dup-summary">
              <p data-testid="dup-summary">{t('dup.head', { n: items.length })} {safe.length > 0 && review.length > 0 ? t('dup.head.split', { safe: safe.length, review: review.length }) : safe.length > 0 ? t('dup.head.allSafe', { safe: safe.length }) : t('dup.head.allReview', { review: review.length })}</p>
              {safe.length > 0 && <Tooltip label={t('dup.resolveAllSafe.tip')}><button type="button" className="primary" data-testid="resolve-all-safe" onClick={() => setDialog({ type: 'all_safe' })}>{t('dup.resolveAllSafe', { n: safe.length })}</button></Tooltip>}
            </div>
          )}
          {review.length > 0 && (
            <section className="dup-kind" aria-labelledby="dup-k-review" data-testid="dup-review-section">
              <h2 id="dup-k-review">{t('dup.section.review')}<span className="dup-total">{review.length}</span></h2>
              <p className="dup-note">{t('dup.section.review.sub')}</p>
              {review.map(card)}
            </section>
          )}
          {[...KINDS, ...new Set(safe.map(i => i.kind).filter(k => !(KINDS as readonly string[]).includes(k)))].map(k => {
            const list = safe.filter(i => i.kind === k); if (!list.length) return null;
            return (
              <section key={k} className="dup-kind" aria-labelledby={`dup-k-${k}`}>
                <h2 id={`dup-k-${k}`}>{kindLabel(k)}{kindGloss(k) && <small> · {kindGloss(k)}</small>}<span className="dup-total">{list.length}</span></h2>
                {list.map(card)}
              </section>
            );
          })}
        </>}
      </div>
      {dialog && <ResolveDialog api={api} scope={dialog} q={q} choices={choices} onClose={() => setDialog(null)} onDone={onDone} />}
      {result && <ResultDialog result={result} onClose={() => setResult(null)} onUndo={result.done.length ? () => { const ids = result.done.map(d => d.id); setResult(null); void undoWithFeedback(api, ids); } : undefined} />}
    </section>
  );
}

