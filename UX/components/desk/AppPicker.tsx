import { useEffect, useMemo, useState } from 'react';
import type { Api, CaseAppDto, InstalledAppDto } from '../../api';
import { t, tn } from '../../i18n';
import { useAppPickerStore } from '../../store/appPicker';
import { useNoticeStore } from '../../store/notices';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { TERM_NAMES, TermTip } from '../ui/terms';
import './appPicker.css';

/** App nên hiện trước (khớp bundleId/tên): nhóm thiết kế mà font app-gắn-Case có ích nhất. */
const PREFERRED = [/illustrator/i, /indesign/i, /photoshop/i, /affinity/i, /figma/i];
/** Trình duyệt/app web: không gắn được (chỉ thấy font hệ thống, không có tín hiệu "app web đang mở"), nên không đề xuất. */
export const BROWSERS = /^(com\.apple\.safari|com\.google\.chrome|org\.mozilla\.firefox|com\.microsoft\.edgemac|company\.thebrowser\.browser|com\.brave\.browser|com\.operasoftware\.opera|com\.vivaldi\.vivaldi)/i;
export const isBrowserApp = (a: { bundleId: string }): boolean => BROWSERS.test(a.bundleId);
const rank = (a: { bundleId: string; name: string }): number => { const i = PREFERRED.findIndex(r => r.test(a.bundleId) || r.test(a.name)); return i < 0 ? PREFERRED.length : i; };

/** Danh sách hiển thị: app đã gắn nhưng không còn cài vẫn hiện (để gỡ được); trình duyệt bỏ; ưu tiên nhóm thiết kế rồi theo tên. */
export function pickerRows(installed: readonly InstalledAppDto[], attached: readonly CaseAppDto[]): Array<{ bundleId: string; name: string; installed: boolean; running: boolean }> {
  const rows = installed.filter(a => !isBrowserApp(a)).map(a => ({ bundleId: a.bundleId, name: a.name, installed: true, running: a.running }));
  for (const a of attached) if (!rows.some(r => r.bundleId === a.bundleId)) rows.push({ bundleId: a.bundleId, name: a.name, installed: false, running: false });
  return rows.sort((x, y) => rank(x) - rank(y) || x.name.localeCompare(y.name));
}

function AppPickerDialog({ api, caseId, name, onClose }: { api: Api; caseId: number; name: string; onClose(): void }): React.JSX.Element {
  const [installed, setInstalled] = useState<InstalledAppDto[] | null>(null);
  const [before, setBefore] = useState<CaseAppDto[] | null>(null);
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [q, setQ] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    Promise.all([api.installedApps(), api.caseApps(caseId)]).then(([apps, cur]) => { if (!alive) return; setInstalled(apps); setBefore(cur); setSel(new Set(cur.map(a => a.bundleId))); },
      e => { if (alive) setErr((e as Error)?.message ?? String(e)); });
    return () => { alive = false; };
  }, [api, caseId]);
  const rows = useMemo(() => (installed && before ? pickerRows(installed, before) : []), [installed, before]);
  const shown = rows.filter(r => !q.trim() || r.name.toLowerCase().includes(q.trim().toLowerCase()));
  const dirty = !!before && (sel.size !== before.length || before.some(a => !sel.has(a.bundleId)));
  const toggle = (id: string) => setSel(s => { const n = new Set(s); if (n.has(id)) n.delete(id); else n.add(id); return n; });
  const save = async () => {
    if (!before) return;
    const next: CaseAppDto[] = rows.filter(r => sel.has(r.bundleId)).map(r => ({ bundleId: r.bundleId, name: r.name }));
    setBusy(true);
    try {
      await api.caseSetApps(caseId, next);
      onClose();
      useNoticeStore.getState().push({
        kind: 'info', text: next.length ? t('app.saved', { n: next.length, name }) : t('app.cleared', { name }),
        action: { label: t('dnd.undo'), run: () => { api.caseSetApps(caseId, before).then(() => useNoticeStore.getState().push({ kind: 'info', text: t('app.undone', { name }) }), e => useNoticeStore.getState().push({ kind: 'error', text: t('app.fail', { msg: (e as Error)?.message ?? String(e) }) })); } },
      });
    } catch (e) { setErr((e as Error)?.message ?? String(e)); setBusy(false); }
  };
  return (
    <Modal title={t('app.title', { name })} onClose={onClose} className="fa-apppicker">
      <p>{t('app.hint')} <TermTip term="scope_session"><span className="fa-termword" data-term="scope_session" tabIndex={0}>{TERM_NAMES.scope_session}</span></TermTip></p>
      <input className="fa-apppicker-q" data-testid="app-search" type="search" value={q} onChange={e => setQ(e.target.value)} placeholder={t('app.search')} aria-label={t('app.search')} disabled={!installed} />
      <ul className="fa-apppicker-list" data-testid="app-list" role="group" aria-label={t('app.list')}>
        {!installed && !err && <li className="fa-apppicker-empty">{t('app.loading')}</li>}
        {installed && shown.length === 0 && <li className="fa-apppicker-empty">{t('app.none')}</li>}
        {shown.map(r => (
          <li key={r.bundleId}>
            <label data-testid="app-row" data-bundle={r.bundleId}>
              <input type="checkbox" checked={sel.has(r.bundleId)} onChange={() => toggle(r.bundleId)} />
              <span className="fa-apppicker-name">{r.name}</span>
              <span className="fa-apppicker-state">{!r.installed ? t('app.notInstalled') : r.running ? t('app.running') : ''}</span>
            </label>
          </li>
        ))}
      </ul>
      <p className="fa-apppicker-web" data-testid="app-web-note"><NoteMark inline kind="info" text={t('app.web.tip')} /> {t('app.web')}</p>
      {err && <p role="alert" className="fa-apppicker-err">{t('app.fail', { msg: err })}</p>}
      <p className="fa-apppicker-count">{tn('app.count', { n: sel.size })}</p>
      <div className="fa-modal-actions">
        <button type="button" onClick={onClose}>{t('app.cancel')}</button>
        <button type="button" className="primary" data-testid="app-save" disabled={busy || !before || !dirty} onClick={() => void save()}>{t('app.save')}</button>
      </div>
    </Modal>
  );
}

/** Gắn một lần ở App: hiện hộp thoại khi `useAppPickerStore.target` có giá trị. */
export function AppPickerHost({ api }: { api: Api }): React.JSX.Element | null {
  const target = useAppPickerStore(s => s.target); const close = useAppPickerStore(s => s.close);
  if (!target) return null;
  return <AppPickerDialog key={target.caseId} api={api} caseId={target.caseId} name={target.name} onClose={close} />;
}
