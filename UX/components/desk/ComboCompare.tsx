import { useEffect, useState } from 'react';
import type { Api } from '../../api';
import type { CaseCompareDto, ComboColumnDto, ComboDto, ComboRoleDto, FaceRowDto } from '../../bindings';
import { t } from '../../i18n';
import { useFontFamily } from '../detail/useFontFamily';
import { NoteMark } from '../ui/noteMark';
import { Tooltip } from '../ui/Tooltip';
import { ROLE_ORDER, isHeadingLike } from './deskRoles';

const comboTitle = (c: { idx: number; name: string | null }): string => `${t('combo.label')} ${c.idx}${c.name && c.name !== `Combo ${c.idx}` ? ` · ${c.name}` : ''}`;
const nameOf = (r: FaceRowDto | null) => (r ? `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}` : '—');
const roleName = (r: ComboRoleDto) => t(`role.${r}` as 'role.display');
const slotRow = (c: ComboDto, roles: readonly ComboRoleDto[]) => roles.map(r => c.slots.find(s => s.role === r)).find(s => s && !s.missing)?.row ?? null;

/** Trang thu nhỏ: tiêu đề bằng font nhóm heading, đoạn văn bằng font nhóm body của Combo. */
function MiniPage({ api, combo }: { api: Api; combo: ComboDto }): React.JSX.Element {
  const head = slotRow(combo, ROLE_ORDER.filter(isHeadingLike)); const body = slotRow(combo, ['body', 'caption']);
  const hf = useFontFamily(api, head); const bf = useFontFamily(api, body);
  const css = (fam: string | null, row: FaceRowDto | null) => ({ fontFamily: fam ? `"${fam}", var(--font-ui)` : undefined, fontWeight: row?.weightClass ?? undefined, fontStyle: row?.isItalic ? 'italic' as const : undefined });
  return (
    <div className="fa-cc-page" data-testid="cc-page">
      <div style={{ ...css(hf, head), fontSize: 20, lineHeight: 1.15 }}>{t('desk.pageHeading')}</div>
      <p style={{ ...css(bf, body), fontSize: 11, lineHeight: 1.5, margin: '6px 0 0' }}>{t('desk.pageBody').slice(0, 120)}</p>
    </div>
  );
}

const reasonText = (r: string | null) => (r ? t(`cc.reason.${r}` as 'cc.reason.missing_font') : '');

/** Compare Combo (06 §3.2): mỗi cột một Combo của Case, mốc = Combo đang dùng; hàng Specimen · Giọng · Quan hệ · VI thấp nhất · Số font · License. */
export function ComboCompare({ api, caseId, currentId, onOpen, onMain }: { api: Api; caseId: number; currentId: number; onOpen(id: number): void; onMain(id: number): void }): React.JSX.Element {
  const [data, setData] = useState<CaseCompareDto | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    let alive = true; setErr(null);
    api.caseCompare(caseId, [], 'vi').then(d => { if (alive) setData(d); }, e => { if (alive) setErr(e?.message ?? String(e)); });
    return () => { alive = false; };
  }, [api, caseId]);
  if (err) return <p role="alert" className="fa-desk-err">{t('desk.loadFail', { msg: err })}</p>;
  if (!data) return <p className="fa-desk-hint">{t('compare.loading')}</p>;
  // Mốc đứng đầu, các Combo còn lại theo idx.
  const cols = [...data.columns].sort((a, b) => (a.combo.id === currentId ? -1 : b.combo.id === currentId ? 1 : a.combo.idx - b.combo.idx));
  return (
    <div className="fa-cc" data-testid="combo-compare">
      <table>
        <thead><tr><th />{cols.map(c => (
          <th key={c.combo.id} data-testid="cc-col" data-base={c.combo.id === currentId || undefined}>
            <div className="fa-cc-title">{c.combo.isMain ? '★ ' : ''}{comboTitle(c.combo)}{c.combo.id === currentId && <small> · {t('compare.base')}</small>}</div>
            <div className="fa-cc-actions">
              <button type="button" data-testid="cc-open" onClick={() => onOpen(c.combo.id)}>{t('cc.open')}</button>
              <button type="button" data-testid="cc-main" disabled={c.combo.isMain} onClick={() => onMain(c.combo.id)}>{t('cc.useThis')}</button>
            </div>
          </th>))}</tr></thead>
        <tbody>
          <tr><th scope="row">{t('cc.specimen')}</th>{cols.map(c => <td key={c.combo.id}><MiniPage api={api} combo={c.combo} /></td>)}</tr>
          {ROLE_ORDER.filter(r => cols.some(c => c.combo.slots.some(s => s.role === r))).map(r => (
            <tr key={r} data-role={r}><th scope="row"><Tooltip label={t(`role.${r}.gloss` as 'role.display.gloss')}><span data-term={`role.${r}`}>{roleName(r)}</span></Tooltip></th>{cols.map(c => {
              const s = c.combo.slots.find(x => x.role === r);
              return <td key={c.combo.id}>{s ? <>{nameOf(s.row)}{s.missing && <NoteMark inline text={t('desk.missing')} />}</> : '—'}</td>;
            })}</tr>
          ))}
          <tr data-row="relations"><th scope="row">{t('cc.relations')}</th>{cols.map(c => (
            <td key={c.combo.id} data-testid="cc-rel">
              {c.relations.length === 0 ? <span className="fa-cc-na">{t('cc.noRelations')}</span> : c.relations.map(r => (
                <div key={`${r.from}-${r.to}`}>{roleName(r.from)}↔{roleName(r.to)}: {r.assessment ? <span title={r.assessment.summary}>{r.assessment.labels.map(l => l.symbol).join('') || '○'}</span> : <span className="fa-cc-na">{reasonText(r.reason)}</span>}</div>
              ))}
            </td>))}</tr>
          <tr data-row="vi"><th scope="row">{t('cc.vi')}</th>{cols.map(c => (
            <td key={c.combo.id} data-testid="cc-vi">
              {c.worstViMissing === null ? '—' : c.worstViMissing === 0 ? t('strip.viOk') : t('strip.viMissing', { n: c.worstViMissing })}
              {c.missingCount > 0 && <NoteMark inline kind="warn" text={t('cc.partial', { n: c.missingCount })} />}
            </td>))}</tr>
          <tr data-row="fonts"><th scope="row">{t('cc.fonts')}</th>{cols.map(c => <td key={c.combo.id}>{c.fontCount}</td>)}</tr>
          <tr data-row="license"><th scope="row">License</th>{cols.map(c => <td key={c.combo.id}>{c.licenses.length ? c.licenses.join(' · ') : '—'}</td>)}</tr>
        </tbody>
      </table>
    </div>
  );
}
