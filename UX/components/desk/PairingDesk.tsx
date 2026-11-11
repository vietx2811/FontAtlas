import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Modal } from '../Modal';
import type { Api, FaceKeyDto, PairCandidateDto } from '../../api';
import type { CaseDto, ComboDto, ComboRoleDto, FaceRowDto } from '../../bindings';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { DEFAULT_SAMPLE_TEXT, useUiStore, type DeskState } from '../../store/ui';
import { useFontFamily } from '../detail/useFontFamily';
import { FaceTitle } from '../ui/FaceTitle';
import { NoteMark } from '../ui/noteMark';
import { Tooltip } from '../ui/Tooltip';
import { ROLE_ORDER, anchorPairRole, anchorRoles, isHeadingLike } from './deskRoles';
import { ComboCompare } from './ComboCompare';
import { ComboStepper } from './ComboStepper';
import { FontCompare } from './FontCompare';
import { DetailStrip } from './DetailStrip';
import './desk.css';

const faceOf = (r: { hashHex: string; faceIndex: number }): FaceKeyDto => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });
const same = (a: FaceKeyDto | null | undefined, b: FaceKeyDto | null | undefined) => !!a && !!b && a.hashHex === b.hashHex && a.faceIndex === b.faceIndex;
const nameOf = (r: FaceRowDto | null): string => (r ? `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}` : '—');
const keyStr = (f: FaceKeyDto | null | undefined) => (f ? `${f.hashHex}:${f.faceIndex}` : '');
const roleName = (r: ComboRoleDto) => t(`role.${r}` as 'role.display');
type Trial = { face: FaceKeyDto; row: FaceRowDto; summary: string; fromPeek?: boolean };

/** Pairing Desk (lát 1 của 2b): cột vai · Cuộn chọn (nguồn Ghép) · trang mẫu. */
export function PairingDesk({ api, desk }: { api: Api; desk: DeskState }): React.JSX.Element {
  const closeDesk = useUiStore(s => s.closeDesk);
  const openDesk = useUiStore(s => s.openDesk);
  const openCompare = useUiStore(s => s.openCompare);
  const [stripCollapsed, setStripCollapsed] = useState(false);
  const [mode, setMode] = useState<'pairing' | 'fonts' | 'combos'>('pairing');
  const [cmpRev, setCmpRev] = useState(0);
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

  const anchorKey = keyStr(anchor?.key); const trialKey = keyStr(desk.trial);
  const selRef = useRef<{ role: ComboRoleDto; key: string } | null>(null);
  const mode0 = data?.settings.mode ?? 'safe'; const vi0 = data?.settings.vi ?? true; const ready = !!data && !!combo;
  // Ứng viên cho vai đang chọn: đầu danh sách là `trial` từ peek (nếu có), sau đó pairCandidates quanh font neo.
  const candRole = useRef(role);
  useEffect(() => {
    if (candRole.current !== role) { candRole.current = role; setCands([]); setIdx(-1); } // đổi vai: bỏ ứng viên của vai cũ tới khi vai mới về
    if (!ready || !data) return;
    let alive = true; setCandError(null);
    const base: Promise<Trial[]> = anchor
      ? api.pairCandidates(anchor.key, { role: anchorPairRole(role), mode: mode0, projectVi: vi0, k: 20, collectionId: null, lang: 'vi' })
        .then((c: PairCandidateDto[]) => c.filter(x => !same(x.face, anchor.key)).map(x => ({ face: x.face, row: x.row, summary: x.summary })))
      : Promise.resolve([]);
    const peek: Promise<Trial | null> = desk.trial && desk.role === role
      ? api.queryFaces({ faceKeys: [desk.trial], limit: 1 }).then(r => (r.items[0] ? { face: desk.trial!, row: r.items[0], summary: '', fromPeek: true } : null), () => null)
      : Promise.resolve(null);
    Promise.all([base, peek]).then(([list, p]) => {
      if (!alive) return;
      const merged = p ? [p, ...list.filter(x => !same(x.face, p.face))] : list;
      // Giữ font đang chọn theo khoá khi danh sách nạp lại (đặt font, đổi tên Combo…): không nhảy về đầu.
      const prev = selRef.current && selRef.current.role === role ? merged.findIndex(x => keyStr(x.face) === selRef.current!.key) : -1;
      setCands(merged); setIdx(prev >= 0 ? prev : merged.length ? 0 : -1);
    }, e => { if (alive) { setCands([]); setIdx(-1); setCandError(e?.message ?? String(e)); } });
    return () => { alive = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api, ready, mode0, vi0, anchorKey, role, trialKey]);

  const trial = idx >= 0 ? cands[idx] : undefined;
  useEffect(() => { selRef.current = trial ? { role, key: keyStr(trial.face) } : null; }, [trial, role]);
  const current = slotOf(role);
  // Trang: tiêu đề dùng font của nhóm heading, đoạn văn dùng font nhóm body; vai đang thử thay font hiện tại.
  const headingFace = (isHeadingLike(role) ? trial?.row : undefined) ?? ROLE_ORDER.filter(isHeadingLike).map(r => slotOf(r)?.row).find(Boolean) ?? null;
  const bodyFace = (!isHeadingLike(role) ? trial?.row : undefined) ?? (['body', 'caption'] as const).map(r => slotOf(r)?.row).find(Boolean) ?? null;
  const headFam = useFontFamily(api, headingFace);
  const bodyFam = useFontFamily(api, bodyFace);
  const css = (fam: string | null, row: FaceRowDto | null) => ({ fontFamily: fam ? `"${fam}", var(--font-ui)` : undefined, fontWeight: row?.weightClass ?? undefined, fontStyle: row?.isItalic ? 'italic' as const : undefined });

  const inUseSame = same(current?.key, trial?.face);
  const placeFace = useCallback(async (face: FaceKeyDto, row: FaceRowDto) => {
    if (!combo) return;
    try {
      const next = await api.setComboSlot(combo.id, role, face);
      setData(d => (d ? { ...d, combos: d.combos.map(c => (c.id === next.id ? next : c)) } : d));
      useNoticeStore.getState().push({ kind: 'info', text: t('desk.placed', { font: nameOf(row), role: roleName(role) }) });
    } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('desk.opFail', { msg: e instanceof Error ? e.message : String(e) }) }); }
  }, [api, combo, role]);
  const place = useCallback(async () => { if (trial) await placeFace(trial.face, trial.row); }, [placeFace, trial]);

  const fail = (e: unknown) => useNoticeStore.getState().push({ kind: 'error', text: t('desk.opFail', { msg: e instanceof Error ? e.message : String(e) }) });
  const goCombo = useCallback((id: number) => { openDesk({ caseId: desk.caseId, comboId: id, role, trial: null }); }, [openDesk, desk.caseId, role]);
  const [confirmDel, setConfirmDel] = useState(false);
  const comboOps = {
    go: goCombo,
    create: () => { void api.createCombo(desk.caseId).then(async c => { setData(await api.getCase(desk.caseId)); goCombo(c.id); }).catch(fail); },
    duplicate: () => { if (combo) void api.createCombo(desk.caseId, undefined, combo.id).then(async c => { setData(await api.getCase(desk.caseId)); goCombo(c.id); }).catch(fail); },
    rename: (name: string) => { if (combo) void api.updateCombo(combo.id, { name }).then(async () => setData(await api.getCase(desk.caseId))).catch(fail); },
    makeMain: () => { if (combo) comboOps.makeMainOf(combo.id); },
    makeMainOf: (id: number) => { void api.updateCombo(id, { isMain: true }).then(async () => { setData(await api.getCase(desk.caseId)); setCmpRev(n => n + 1); }).catch(fail); },
    remove: () => setConfirmDel(true),
    removeNow: async () => {
      if (!combo || !data) return;
      const pos = data.combos.findIndex(c => c.id === combo.id);
      try {
        await api.deleteCombo(combo.id);
        const fresh = await api.getCase(desk.caseId); setData(fresh);
        const next = fresh.combos[Math.max(0, Math.min(pos - 1, fresh.combos.length - 1))];
        if (next) goCombo(next.id);
      } catch (e) { fail(e); }
    },
  };
  const step = (d: -1 | 1) => { if (!data || !combo) return; const i = data.combos.findIndex(c => c.id === combo.id); const n = data.combos[i + d]; if (n) goCombo(n.id); };
  const selectRole = (r: ComboRoleDto) => { if (combo && r !== role) openDesk({ caseId: desk.caseId, comboId: combo.id, role: r, trial: null }); };

  useEffect(() => { root.current?.focus(); }, []);
  // Nút vừa bị vô hiệu (‹ ở Combo 1, Xoá…) làm focus rơi về body: đưa về Desk để phím tắt còn sống.
  useEffect(() => {
    if (confirmDel) return; // hộp xác nhận đang giữ focus: không cướp
    const a = document.activeElement as HTMLElement | null;
    if (!a || a === document.body || !root.current?.contains(a) || (a as HTMLButtonElement).disabled) root.current?.focus();
  }, [desk.comboId, mode, confirmDel]);
  const leave = useCallback(() => { closeDesk(); requestAnimationFrame(() => (document.querySelector('[data-testid=open-desk]') as HTMLElement | null)?.focus()); }, [closeDesk]);
  const cmpFaces = trial ? (current && !current.missing && !inUseSame ? [current.key, trial.face] : [trial.face]) : [];
  const compareReason = cmpFaces.length < 2 ? t('desk.compareNeed') : null;
  const toCompare = () => { if (!compareReason) openCompare(cmpFaces); };
  const onKey = (e: React.KeyboardEvent) => {
    if (confirmDel) return; // hộp xác nhận mở: không phím tắt nào của Desk chạy
    const tgt = e.target as HTMLElement;
    // Sự kiện nổi bọt từ portal (menu, hộp đổi tên) hoặc từ ô nhập không thuộc phím tắt của Desk.
    if (!root.current?.contains(tgt) || tgt.closest('input, textarea, select, [contenteditable]')) return;
    const onList = tgt === root.current || !!tgt.closest('[role=option]');
    if (e.key === 'Escape') { e.preventDefault(); leave(); }
    else if (e.key === 'ArrowDown' && mode === 'pairing') { e.preventDefault(); setIdx(i => Math.min(cands.length - 1, i + 1)); }
    else if (e.key === 'ArrowUp' && mode === 'pairing') { e.preventDefault(); setIdx(i => Math.max(0, i - 1)); }
    else if (e.key === 'Enter' && onList && !e.repeat && mode === 'pairing' && trial && !inUseSame) { e.preventDefault(); void place(); }
    else if ((e.key === '[' || e.key === ']') && !e.metaKey && !e.ctrlKey && !e.altKey) { e.preventDefault(); step(e.key === '[' ? -1 : 1); }
    else if ((e.key === 'i' || e.key === 'I') && !e.metaKey && !e.ctrlKey && !e.altKey && mode === 'pairing') { e.preventDefault(); setStripCollapsed(c => !c); }
    else if ((e.key === 'c' || e.key === 'C') && e.shiftKey && trial && !compareReason) { e.preventDefault(); toCompare(); }
  };

  const shownRoles = ROLE_ORDER.filter(r => slotOf(r) || r === role);

  return (
    <div ref={root} className="fa-desk" data-testid="pairing-desk" tabIndex={-1} onKeyDown={onKey}>
      <header className="fa-desk-head">
        <Tooltip label={t('desk.backTip')} shortcut="Esc"><button type="button" className="fa-desk-back" data-testid="desk-back" onClick={leave}>← {t('desk.back')}</button></Tooltip>
        <h1>{t('desk.title')}</h1>
        {data && <span className="fa-desk-case" data-testid="desk-case">{t('desk.case')}: {data.name}</span>}
        <span className="fa-desk-mode" role="group" aria-label={t('desk.mode.tip')}>
          <button type="button" data-testid="mode-pairing" aria-pressed={mode === 'pairing'} onClick={() => setMode('pairing')}>{t('desk.mode.pairing')}</button>
          <button type="button" data-testid="mode-fonts" aria-pressed={mode === 'fonts'} onClick={() => setMode('fonts')}>{t('desk.mode.fonts')}</button>
          <button type="button" data-testid="mode-combos" aria-pressed={mode === 'combos'} onClick={() => setMode('combos')}>{t('desk.mode.combos')}</button>
        </span>
        {data && combo && <ComboStepper data={data} combo={combo} ops={comboOps} />}
      </header>
      {error && <p role="alert" className="fa-desk-err">{t('desk.loadFail', { msg: error })}</p>}
      {mode === 'fonts' && data && combo ? (
        <FontCompare key={`${combo.id}:${role}`} api={api} role={role}
          base={current && !current.missing ? { face: current.key, row: current.row } : null}
          partner={(anchor?.row ?? null)} suggestions={cands.map(c => ({ face: c.face, row: c.row }))}
          onPlace={f => { void placeFace(f.face, f.row); }}
          onTry={f => { setMode('pairing'); openDesk({ caseId: desk.caseId, comboId: combo.id, role, trial: f.face }); }} />
      ) : mode === 'combos' && data && combo ? (
        <ComboCompare key={cmpRev} api={api} caseId={desk.caseId} currentId={combo.id} onOpen={id => { setMode('pairing'); goCombo(id); }} onMain={id => comboOps.makeMainOf(id)} />
      ) : (
      <div className="fa-desk-body">
        <section className="fa-desk-roles" aria-label={t('desk.roles')}>
          <h2>{t('desk.roles')}</h2>
          <ul>
            {shownRoles.map(r => {
              const s = slotOf(r);
              return (
                <li key={r}><button type="button" className="fa-desk-role" data-testid="desk-role" data-role={r} aria-pressed={r === role} onClick={() => selectRole(r)}>
                  <Tooltip label={t(`role.${r}.gloss` as 'role.display.gloss')}><span className="fa-desk-role-name" data-term={`role.${r}`}>{roleName(r)}</span></Tooltip>
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
            <div className="fa-desk-h" data-testid="desk-heading" style={{ ...css(headFam, headingFace), fontSize: 34, lineHeight: 1.15 }}>{sample.trim() && sample !== DEFAULT_SAMPLE_TEXT ? sample : t('desk.pageHeading')}</div>
            <p className="fa-desk-p" data-testid="desk-body" style={{ ...css(bodyFam, bodyFace), fontSize: 16, lineHeight: 1.6 }}>{t('desk.pageBody')}</p>
          </div>
          <div className="fa-desk-strip">
            {trial && <span className="fa-desk-trying" data-testid="desk-trying">{t('desk.trying')}: <b>{nameOf(trial.row)}</b></span>}
            {current && <span className="fa-desk-using">{t('desk.inUse')}: {nameOf(current.row)}</span>}
            <button type="button" className="fa-desk-place" data-testid="desk-place" aria-disabled={!trial || inUseSame} onClick={() => { if (trial && !inUseSame) void place(); }}>↵ {t('desk.place')}</button>
          </div>
          <DetailStrip api={api} inUse={current && !current.missing ? current.row : null} trial={trial?.row ?? null} collapsed={stripCollapsed} onToggle={() => setStripCollapsed(c => !c)} onCompare={toCompare} compareReason={compareReason} />
        </section>
      </div>
      )}
      {confirmDel && combo && (
        <Modal title={t('combo.delete.title', { n: combo.idx })} onClose={() => setConfirmDel(false)}>
          <p>{t('combo.delete.body', { n: String(combo.slots.length) })}</p>
          <div className="fa-modal-actions">
            <button type="button" data-testid="combo-delete-cancel" onClick={() => setConfirmDel(false)}>{t('combo.cancel')}</button>
            <button type="button" className="primary" data-testid="combo-delete-confirm" onClick={() => { setConfirmDel(false); void comboOps.removeNow(); }}>{t('combo.delete')}</button>
          </div>
        </Modal>
      )}
    </div>
  );
}
