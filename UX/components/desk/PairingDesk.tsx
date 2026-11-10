import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Api, FaceKeyDto, PairCandidateDto } from '../../api';
import type { CaseDto, ComboDto, ComboRoleDto, FaceRowDto } from '../../bindings';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore, type DeskState } from '../../store/ui';
import { useFontFamily } from '../detail/useFontFamily';
import { PARAGRAPH } from '../detail/tabs/SpecimenTab';
import { FaceTitle } from '../ui/FaceTitle';
import { NoteMark } from '../ui/noteMark';
import { Tooltip } from '../ui/Tooltip';
import { ROLE_ORDER, anchorPairRole, anchorRoles, isHeadingLike } from './deskRoles';
import './desk.css';

const faceOf = (r: { hashHex: string; faceIndex: number }): FaceKeyDto => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });
const same = (a: FaceKeyDto | null | undefined, b: FaceKeyDto | null | undefined) => !!a && !!b && a.hashHex === b.hashHex && a.faceIndex === b.faceIndex;
const nameOf = (r: FaceRowDto | null): string => (r ? `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}` : '—');
const roleName = (r: ComboRoleDto) => t(`role.${r}` as 'role.display');
type Trial = { face: FaceKeyDto; row: FaceRowDto; summary: string; fromPeek?: boolean };

/** Pairing Desk (lát 1 của 2b): cột vai · Cuộn chọn (nguồn Ghép) · trang mẫu. */
export function PairingDesk({ api, desk }: { api: Api; desk: DeskState }): React.JSX.Element {
  const closeDesk = useUiStore(s => s.closeDesk);
  const openDesk = useUiStore(s => s.openDesk);
  const sample = useUiStore(s => s.sample);
  const [data, setData] = useState<CaseDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [cands, setCands] = useState<Trial[]>([]);
  const [candError, setCandError] = useState<string | null>(null);
  const [idx, setIdx] = useState<number>(-1);
  const root = useRef<HTMLDivElement>(null);
  const role = desk.role;

  useEffect(() => {
    let alive = true; setError(null);
    api.getCase(desk.caseId).then(c => { if (alive) setData(c); }, e => { if (alive) setError(e?.message ?? String(e)); });
    return () => { alive = false; };
  }, [api, desk.caseId]);

  const combo: ComboDto | undefined = useMemo(() => data?.combos.find(c => c.id === desk.comboId) ?? data?.combos[0], [data, desk.comboId]);
  const slotOf = useCallback((r: ComboRoleDto) => combo?.slots.find(s => s.role === r), [combo]);
  const anchor = useMemo(() => { for (const r of anchorRoles(role)) { const s = slotOf(r); if (s && !s.missing) return s; } return undefined; }, [role, slotOf]);

  // Ứng viên cho vai đang chọn: đầu danh sách là `trial` từ peek (nếu có), sau đó pairCandidates quanh font neo.
  useEffect(() => {
    if (!data || !combo) return;
    let alive = true; setCandError(null);
    const base: Promise<Trial[]> = anchor
      ? api.pairCandidates(anchor.key, { role: anchorPairRole(role), mode: data.settings.mode ?? 'safe', projectVi: data.settings.vi, k: 20, collectionId: null, lang: 'vi' })
        .then((c: PairCandidateDto[]) => c.map(x => ({ face: x.face, row: x.row, summary: x.summary })))
      : Promise.resolve([]);
    const peek: Promise<Trial | null> = desk.trial && desk.role === role
      ? api.queryFaces({ faceKeys: [desk.trial], limit: 1 }).then(r => (r.items[0] ? { face: desk.trial!, row: r.items[0], summary: '', fromPeek: true } : null), () => null)
      : Promise.resolve(null);
    Promise.all([base, peek]).then(([list, p]) => {
      if (!alive) return;
      const merged = p ? [p, ...list.filter(x => !same(x.face, p.face))] : list;
      setCands(merged); setIdx(merged.length ? 0 : -1);
    }, e => { if (alive) { setCands([]); setIdx(-1); setCandError(e?.message ?? String(e)); } });
    return () => { alive = false; };
  }, [api, data, combo, anchor, role, desk.trial]);

  const trial = idx >= 0 ? cands[idx] : undefined;
  const current = slotOf(role);
  // Trang: tiêu đề dùng font của nhóm heading, đoạn văn dùng font nhóm body; vai đang thử thay font hiện tại.
  const headingFace = (isHeadingLike(role) ? trial?.row : undefined) ?? ROLE_ORDER.filter(isHeadingLike).map(r => slotOf(r)?.row).find(Boolean) ?? null;
  const bodyFace = (!isHeadingLike(role) ? trial?.row : undefined) ?? (['body', 'caption'] as const).map(r => slotOf(r)?.row).find(Boolean) ?? null;
  const headFam = useFontFamily(api, headingFace);
  const bodyFam = useFontFamily(api, bodyFace);
  const css = (fam: string | null, row: FaceRowDto | null) => ({ fontFamily: fam ? `"${fam}", var(--font-ui)` : undefined, fontWeight: row?.weightClass ?? undefined, fontStyle: row?.isItalic ? 'italic' as const : undefined });

  const place = useCallback(async () => {
    if (!combo || !trial) return;
    try {
      const next = await api.setComboSlot(combo.id, role, trial.face);
      setData(d => (d ? { ...d, combos: d.combos.map(c => (c.id === next.id ? next : c)) } : d));
      useNoticeStore.getState().push({ kind: 'info', text: t('desk.placed', { font: nameOf(trial.row), role: roleName(role) }) });
    } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('peek.openFail', { msg: e instanceof Error ? e.message : String(e) }) }); }
  }, [api, combo, trial, role]);

  const selectRole = (r: ComboRoleDto) => { if (combo && r !== role) openDesk({ caseId: desk.caseId, comboId: combo.id, role: r, trial: null }); };

  useEffect(() => { root.current?.focus(); }, []);
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') { e.preventDefault(); closeDesk(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); setIdx(i => Math.min(cands.length - 1, i + 1)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setIdx(i => Math.max(0, i - 1)); }
    else if (e.key === 'Enter' && trial) { e.preventDefault(); void place(); }
  };

  const shownRoles = ROLE_ORDER.filter(r => slotOf(r) || r === role);
  const inUseSame = same(current?.key, trial?.face);

  return (
    <div ref={root} className="fa-desk" data-testid="pairing-desk" tabIndex={-1} onKeyDown={onKey}>
      <header className="fa-desk-head">
        <Tooltip label={t('desk.backTip')} shortcut="Esc"><button type="button" className="fa-desk-back" data-testid="desk-back" onClick={closeDesk}>← {t('desk.back')}</button></Tooltip>
        <h1>{t('desk.title')}</h1>
        {data && <span className="fa-desk-case" data-testid="desk-case">{t('desk.case')}: {data.name}{combo ? ` · Combo ${combo.idx}/${data.combos.length}` : ''}</span>}
      </header>
      {error && <p role="alert" className="fa-desk-err">{t('desk.loadFail', { msg: error })}</p>}
      <div className="fa-desk-body">
        <section className="fa-desk-roles" aria-label={t('desk.roles')}>
          <h2>{t('desk.roles')}</h2>
          <ul>
            {shownRoles.map(r => {
              const s = slotOf(r);
              return (
                <li key={r}><button type="button" className="fa-desk-role" data-testid="desk-role" data-role={r} aria-pressed={r === role} onClick={() => selectRole(r)}>
                  <span className="fa-desk-role-name" data-term={`role.${r}`} title={t(`role.${r}.gloss` as 'role.display.gloss')}>{roleName(r)}</span>
                  {s
                    ? <FaceTitle api={api} face={s.row} className="fa-desk-role-font">{nameOf(s.row)}</FaceTitle>
                    : <span className="fa-desk-role-empty">{t('desk.roleEmpty')}</span>}
                  {s?.missing && <NoteMark inline text={t('desk.missing')} />}
                </button></li>
              );
            })}
          </ul>
        </section>
        <section className="fa-desk-picker" aria-label={t('desk.picker')}>
          <h2>{t('desk.picker')} · {roleName(role)} <small>{t('desk.source.pair')}</small></h2>
          {!anchor && <p className="fa-desk-hint" data-testid="desk-noanchor">{t('desk.noAnchor', { role: roleName(anchorRoles(role)[0]) })}</p>}
          {candError && <p role="alert" className="fa-desk-err">{t('desk.candFail', { msg: candError })}</p>}
          {anchor && !candError && cands.length === 0 && <p className="fa-desk-hint">{t('desk.noCands')}</p>}
          <ul className="fa-desk-cands" role="listbox" aria-label={t('desk.picker')}>
            {cands.map((c, i) => (
              <li key={`${c.face.hashHex}:${c.face.faceIndex}`} role="option" aria-selected={i === idx} data-testid="desk-cand" data-trial={c.fromPeek ? 'peek' : undefined}
                onPointerEnter={() => setIdx(i)} onClick={() => { setIdx(i); }} onDoubleClick={() => { setIdx(i); void place(); }}>
                <FaceTitle api={api} face={c.row} className="fa-desk-cand-name">{nameOf(c.row)}</FaceTitle>
                {c.summary && <span className="fa-desk-cand-sum">{c.summary}</span>}
              </li>
            ))}
          </ul>
          <p className="fa-desk-keys">{t('desk.pickerHint')}</p>
        </section>
        <section className="fa-desk-page" aria-label={t('desk.sampleLabel')}>
          <div className="fa-desk-sheet">
            <div className="fa-desk-h" data-testid="desk-heading" style={{ ...css(headFam, headingFace), fontSize: 34, lineHeight: 1.15 }}>{sample.trim() || t('desk.pageHeading')}</div>
            <p className="fa-desk-p" data-testid="desk-body" style={{ ...css(bodyFam, bodyFace), fontSize: 16, lineHeight: 1.6 }}>{PARAGRAPH}</p>
          </div>
          <div className="fa-desk-strip">
            {trial && <span className="fa-desk-trying" data-testid="desk-trying">{t('desk.trying')}: <b>{nameOf(trial.row)}</b></span>}
            {current && <span className="fa-desk-using">{t('desk.inUse')}: {nameOf(current.row)}</span>}
            <button type="button" className="fa-desk-place" data-testid="desk-place" disabled={!trial || inUseSame} onClick={() => void place()}>↵ {t('desk.place')}</button>
          </div>
        </section>
      </div>
    </div>
  );
}
