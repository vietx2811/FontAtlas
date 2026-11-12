import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Modal } from '../Modal';
import { useActivationFlow } from '../activation/useActivationFlow';
import type { Api, FaceKeyDto, PairCandidateDto } from '../../api';
import type { CaseDto, ComboDto, ComboRoleDto, FaceRowDto } from '../../bindings';
import { t, tn } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { defaultSample, useUiStore, type DeskState } from '../../store/ui';
import { useFontFamily } from '../detail/useFontFamily';
import { FaceTitle } from '../ui/FaceTitle';
import { NoteMark } from '../ui/noteMark';
import { Tooltip } from '../ui/Tooltip';
import { ROLE_ORDER, anchorRoles, isHeadingLike, presetKey, roleLabel } from './deskRoles';
import { RoleList } from './RoleList';
import { PickerBar } from './PickerBar';
import { SOURCES, guessProfile, keepClasses, loadSource, type PickItem, type PickSource, type Profile } from './pickerSources';
import { ComboCompare } from './ComboCompare';
import { ComboStepper } from './ComboStepper';
import { FontCompare } from './FontCompare';
import { DetailStrip } from './DetailStrip';
import './desk.css';

const faceOf = (r: { hashHex: string; faceIndex: number }): FaceKeyDto => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });
const same = (a: FaceKeyDto | null | undefined, b: FaceKeyDto | null | undefined) => !!a && !!b && a.hashHex === b.hashHex && a.faceIndex === b.faceIndex;
const nameOf = (r: FaceRowDto | null): string => (r ? `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}` : '—');
const keyStr = (f: FaceKeyDto | null | undefined) => (f ? `${f.hashHex}:${f.faceIndex}` : '');
/** Số font từ đó hỏi xác nhận trước khi bật hàng loạt (spec C6). */
const CONFIRM_ACTIVATE_FROM = 200;
const roleName = roleLabel;
type Trial = PickItem;

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
  const [all, setAll] = useState<Trial[]>([]);
  const [candError, setCandError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [retry, setRetry] = useState(0);
  const [idx, setIdx] = useState<number>(-1);
  // G5: nguồn theo vai (nhớ trong phiên Desk), lọc class và "Đủ tiếng Việt" giữ khi đổi nguồn.
  const srcByRole = useRef(new Map<string, PickSource>());
  const [source, setSourceState] = useState<PickSource>(() => (desk.trial ? 'pair' : 'role'));
  const setSource = useCallback((s: PickSource, r?: string) => { srcByRole.current.set(r ?? desk.role, s); setSourceState(s); }, [desk.role]);
  const [classes, setClasses] = useState<string[]>([]);
  const [viOnly, setViOnly] = useState(false);
  const viInit = useRef(false);
  const [passSet, setPassSet] = useState<Set<string> | null>(null);
  const [profiles, setProfiles] = useState<Record<string, Profile | null>>({});
  const root = useRef<HTMLDivElement>(null);
  const activation = useActivationFlow(api);
  const [confirmAll, setConfirmAll] = useState(false);
  const [activating, setActivating] = useState(false);
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
  // Đổi vai: nguồn đã nhớ của vai đó, nếu chưa có thì Role fit (Lead chốt), có peek thì Ghép.
  const candRole = useRef(role);
  useEffect(() => {
    if (candRole.current === role) return;
    candRole.current = role; setAll([]); setIdx(-1); // đổi vai: bỏ ứng viên của vai cũ tới khi vai mới về
    setSourceState(srcByRole.current.get(role) ?? (desk.trial ? 'pair' : 'role'));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [role]);
  useEffect(() => { if (data && !viInit.current) { viInit.current = true; setViOnly(!!data.settings.vi); } }, [data]);
  const profile: Profile | null = profiles[role] ?? null;
  const profileGuess = guessProfile(role);
  // Hồ sơ vai: vai preset tự biết; vai tuỳ ý dùng hồ sơ người dùng chọn (chưa chọn → backend trả fit unknown + role_unknown).
  const currentKey = keyStr(slotOf(role)?.key);
  const userPicked = useRef(false);
  useEffect(() => {
    if (!ready || !data) return;
    let alive = true; setCandError(null); setLoading(true);
    const slotCur = slotOf(role);
    const run = async (src: PickSource): Promise<Trial[]> => {
      const list = await loadSource(api, src, { role, profile, anchor: anchor?.key ?? null, current: slotCur && !slotCur.missing ? slotCur.key : null, caseId: desk.caseId, projectVi: vi0, mode: mode0 });
      if (src !== 'pair' || !(desk.trial && desk.role === role)) return list;
      const r = await api.queryFaces({ faceKeys: [desk.trial], limit: 1 }).catch(() => null);
      const p: Trial | null = r?.items[0] ? { face: desk.trial, row: r.items[0], summary: '', fromPeek: true } : null;
      return p ? [p, ...list.filter(x => !same(x.face, p.face))] : list;
    };
    run(source).then(async merged => {
      if (!alive) return;
      const keep = await keepClasses(api, merged, classes).catch(() => null);
      if (!alive) return;
      const prev = selRef.current && selRef.current.role === role ? merged.findIndex(x => keyStr(x.face) === selRef.current!.key) : -1;
      setAll(merged); setPassSet(classes.length ? keep : null); setLoading(false);
      setIdx(prev >= 0 ? prev : merged.length ? 0 : -1);
    }, e => {
      if (!alive) return;
      setLoading(false);
      // Role fit chưa dùng được và người dùng chưa tự chọn nguồn: lùi về Ghép (rồi Đã ghim) như spec.
      if (source === 'role' && !userPicked.current && !srcByRole.current.has(role)) { setSourceState(anchor ? 'pair' : 'pin'); return; }
      setAll([]); setIdx(-1); setCandError(e?.message ?? String(e));
    });
    return () => { alive = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api, ready, mode0, vi0, anchorKey, role, trialKey, source, profile, classes, retry, currentKey]);

  const cands = useMemo(() => all.filter(c => c.fromPeek || ((!viOnly || c.row.supportsVi) && (!passSet || passSet.has(keyStr(c.face))))), [all, viOnly, passSet]);
  const hiddenN = all.length - cands.length;
  // Danh sách đổi do lọc: giữ font đang chọn theo khoá nếu còn, không thì về đầu.
  useEffect(() => {
    setIdx(i => { const k = selRef.current && selRef.current.role === role ? cands.findIndex(x => keyStr(x.face) === selRef.current!.key) : -1; return k >= 0 ? k : cands.length ? Math.min(Math.max(i, 0), cands.length - 1) : -1; });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cands]);

  const current = slotOf(role);
  const pinnedKeys = useMemo(() => new Set((data?.faces ?? []).map(f => keyStr(f.key))), [data]);
  const chooseSource = (src: PickSource) => { userPicked.current = true; setSource(src); };
  const disabledSources: Partial<Record<PickSource, string>> = {
    ...(!anchor ? { pair: t('pick.needAnchor', { role: roleName(anchorRoles(role)[0]) }) } : {}),
    ...(!current || current.missing ? { sim: t('pick.needAnchor', { role: roleName(role) }) } : {}),
  };
  const trial = idx >= 0 ? cands[idx] : undefined;
  useEffect(() => { selRef.current = trial ? { role, key: keyStr(trial.face) } : null; }, [trial, role]);
  // Trang: tiêu đề dùng font của nhóm heading, đoạn văn dùng font nhóm body; vai đang thử thay font hiện tại.
  const headingFace = (isHeadingLike(role) ? trial?.row : undefined) ?? ROLE_ORDER.filter(isHeadingLike).map(r => slotOf(r)?.row).find(Boolean) ?? null;
  const bodyFace = (!isHeadingLike(role) ? trial?.row : undefined) ?? (['body', 'caption'] as const).map(r => slotOf(r)?.row).find(Boolean) ?? null;
  const headFam = useFontFamily(api, headingFace);
  const bodyFam = useFontFamily(api, bodyFace);
  const css = (fam: string | null, row: FaceRowDto | null) => ({ fontFamily: fam ? `"${fam}", var(--font-ui)` : undefined, fontWeight: row?.weightClass ?? undefined, fontStyle: row?.isItalic ? 'italic' as const : undefined });

  const inUseSame = same(current?.key, trial?.face);
  const placeFace = useCallback(async (face: FaceKeyDto, row: FaceRowDto): Promise<boolean> => {
    if (!combo) return false;
    try {
      const next = await api.setComboSlot(combo.id, role, face);
      setData(d => (d ? { ...d, combos: d.combos.map(c => (c.id === next.id ? next : c)) } : d)); setCmpRev(n => n + 1);
      useNoticeStore.getState().push({ kind: 'info', text: t('desk.placed', { font: nameOf(row), role: roleName(role) }) });
      return true;
    } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('desk.opFail', { msg: e instanceof Error ? e.message : String(e) }) }); return false; }
  }, [api, combo, role]);
  const place = useCallback(async () => { if (trial) await placeFace(trial.face, trial.row); }, [placeFace, trial]);

  const fail = (e: unknown) => useNoticeStore.getState().push({ kind: 'error', text: t('desk.opFail', { msg: e instanceof Error ? e.message : String(e) }) });
  const goCombo = useCallback((id: number) => { openDesk({ caseId: desk.caseId, comboId: id, role, trial: null }); }, [openDesk, desk.caseId, role]);
  const [confirmDel, setConfirmDel] = useState(false);
  // Mọi thao tác Combo (tạo/nhân bản/đổi tên/đặt chính/xoá) đọc lại Case và làm mới bảng So sánh Combo.
  const refreshCase = async () => { const c = await api.getCase(desk.caseId); setData(c); setCmpRev(n => n + 1); return c; };
  /** Ghim = thêm vào Case; bỏ ghim = gỡ khỏi Case (không cho khi font đang đặt ở một vai: gỡ sẽ làm vai đó rỗng). */
  const togglePin = async (c: Trial) => {
    const k = keyStr(c.face); const pinned = pinnedKeys.has(k);
    if (pinned && combo?.slots.some(sl => keyStr(sl.key) === k)) { useNoticeStore.getState().push({ kind: 'info', text: t('pick.unpin.inUse') }); return; }
    try {
      if (pinned) await api.removeFromCase(desk.caseId, [c.face]); else await api.addToCase(desk.caseId, [c.face]);
      await refreshCase();
      useNoticeStore.getState().push({ kind: 'info', text: t(pinned ? 'pick.unpin.done' : 'pick.pin.done', { font: nameOf(c.row) }) });
    } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('pick.fail.pin', { msg: e instanceof Error ? e.message : String(e) }) }); }
  };
  const comboOps = {
    go: goCombo,
    create: () => { void api.createCombo(desk.caseId).then(async c => { await refreshCase(); goCombo(c.id); }).catch(fail); },
    duplicate: () => { if (combo) void api.createCombo(desk.caseId, undefined, combo.id).then(async c => { await refreshCase(); goCombo(c.id); }).catch(fail); },
    rename: (name: string) => { if (combo) void api.updateCombo(combo.id, { name }).then(async () => { await refreshCase(); }).catch(fail); },
    makeMain: () => { if (combo) comboOps.makeMainOf(combo.id); },
    makeMainOf: (id: number) => { void api.updateCombo(id, { isMain: true }).then(async () => { await refreshCase(); }).catch(fail); },
    remove: () => setConfirmDel(true),
    removeNow: async () => {
      if (!combo || !data) return;
      const pos = data.combos.findIndex(c => c.id === combo.id);
      try {
        await api.deleteCombo(combo.id);
        const fresh = await refreshCase();
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
    else if (e.key === 'Enter' && onList && !tgt.closest('button') && !e.repeat && mode === 'pairing' && trial && !inUseSame) { e.preventDefault(); void place(); }
    else if (mode === 'pairing' && !e.metaKey && !e.ctrlKey && !e.altKey && /^[1-5]$/.test(e.key)) { e.preventDefault(); const src = SOURCES[Number(e.key) - 1]; if (!disabledSources[src]) chooseSource(src); }
    else if (mode === 'pairing' && !e.metaKey && !e.ctrlKey && !e.altKey && (e.key === 'v' || e.key === 'V')) { e.preventDefault(); setViOnly(v => !v); }
    else if (mode === 'pairing' && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey && (e.key === 'p' || e.key === 'P') && trial) { e.preventDefault(); void togglePin(trial); }
    else if (mode === 'pairing' && !e.metaKey && !e.ctrlKey && !e.altKey && (e.key === '/' || e.key === 'f' || e.key === 'F')) { e.preventDefault(); root.current?.querySelector<HTMLElement>('[data-testid=pick-class]')?.focus(); }
    else if ((e.key === '+' || e.key === '=') && onList && !tgt.closest('button') && !e.repeat && !e.metaKey && !e.ctrlKey && !e.altKey && mode === 'pairing' && trial && !inUseSame) { e.preventDefault(); void place(); }
    else if ((e.key === '[' || e.key === ']') && mode !== 'fonts' && !e.metaKey && !e.ctrlKey && !e.altKey) { e.preventDefault(); step(e.key === '[' ? -1 : 1); }
    else if ((e.key === 'i' || e.key === 'I') && !e.metaKey && !e.ctrlKey && !e.altKey && mode === 'pairing') { e.preventDefault(); setStripCollapsed(c => !c); }
    else if ((e.key === 'c' || e.key === 'C') && e.shiftKey && mode === 'pairing' && trial && !compareReason) { e.preventDefault(); toCompare(); }
  };

  const runActivateAll = async () => { setConfirmAll(false); setActivating(true); try { await activation.activateCase(desk.caseId); } finally { setActivating(false); } };
  // Vai theo thứ tự đã lưu; Combo chưa lưu vai nào thì dùng bộ mặc định (các vai đã có font) và luôn có vai đang chọn.
  const shownRoles: ComboRoleDto[] = (() => {
    if (!combo) return [role];
    const list: ComboRoleDto[] = combo.roles.length ? [...combo.roles] : ROLE_ORDER.filter(r => slotOf(r));
    for (const s of combo.slots) if (!list.includes(s.role)) list.push(s.role);
    if (!list.some(r => r.toLowerCase() === role.toLowerCase())) list.push(role);
    return list;
  })();

  return (
    <div ref={root} className="fa-desk" data-testid="pairing-desk" tabIndex={-1} onKeyDown={onKey}>
      <header className="fa-desk-head">
        <Tooltip label={t('desk.backTip')} shortcut="Esc"><button type="button" className="fa-desk-back" data-testid="desk-back" onClick={leave}>← {t('desk.back')}</button></Tooltip>
        <h1>{t('desk.title')}</h1>
        {data && <span className="fa-desk-case" data-testid="desk-case">{t('desk.case')}: {data.name}</span>}
        {data && (
          <Tooltip label={t('desk.activateAll.tip')}>
            <button type="button" className="fa-desk-activate" data-testid="desk-activate-all" aria-disabled={activating || data.faces.length === 0 || undefined}
              onClick={() => { if (activating || data.faces.length === 0) return; if (data.faces.length > CONFIRM_ACTIVATE_FROM) setConfirmAll(true); else void runActivateAll(); }}>{t('desk.activateAll')}</button>
          </Tooltip>
        )}
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
          onPlace={f => placeFace(f.face, f.row)}
          onTry={f => { setMode('pairing'); openDesk({ caseId: desk.caseId, comboId: combo.id, role, trial: f.face }); }} />
      ) : mode === 'combos' && data && combo ? (
        <ComboCompare key={cmpRev} api={api} caseId={desk.caseId} currentId={combo.id} onOpen={id => { setMode('pairing'); goCombo(id); }} onMain={id => comboOps.makeMainOf(id)} />
      ) : (
      <div className="fa-desk-body">
        <section className="fa-desk-roles" aria-label={t('desk.roles')}>
          <h2>{t('desk.roles')}</h2>
          {combo && <RoleList api={api} combo={combo} roles={shownRoles} role={role} onSelect={selectRole}
            onCombo={(c, sel) => { setData(d => (d ? { ...d, combos: d.combos.map(x => (x.id === c.id ? c : x)) } : d)); setCmpRev(n => n + 1); if (sel && sel !== role) openDesk({ caseId: desk.caseId, comboId: c.id, role: sel, trial: null }); }}
            onRenamed={(from, to) => { if (from === role) openDesk({ caseId: desk.caseId, comboId: combo.id, role: to, trial: null }); }} />}
        </section>
        <section className="fa-desk-picker" aria-label={t('desk.picker')}>
          <h2>{t('pick.title', { role: roleName(role) })}</h2>
          <PickerBar source={source} onSource={chooseSource} disabled={disabledSources} counts={{ pin: pinnedKeys.size }} classes={classes} onClasses={setClasses} viOnly={viOnly} onViOnly={setViOnly}
            shown={cands.length} hidden={hiddenN} profile={profile} profileGuess={profileGuess} showProfile={source === 'role' && !presetKey(role)}
            onProfile={p => setProfiles(m => ({ ...m, [role]: p }))} clearRecent={() => { void api.clearRecentViews().then(() => { setAll([]); setIdx(-1); useNoticeStore.getState().push({ kind: 'info', text: t('pick.recent.cleared') }); }, fail); }} />
          {source === 'pair' && !anchor && <p className="fa-desk-hint" data-testid="desk-noanchor">{t('desk.noAnchor', { role: roleName(anchorRoles(role)[0]) })}</p>}
          {candError && <div role="alert" className="fa-desk-err">{t('pick.fail', { msg: candError })} <button type="button" data-testid="pick-retry" onClick={() => setRetry(n => n + 1)}>{t('pick.retry')}</button></div>}
          {loading && all.length === 0 && !candError && <div className="fa-pick-loading" data-testid="pick-loading" aria-busy="true" role="status"><span className="fa-pick-skel" /><span className="fa-pick-skel" /><span className="fa-pick-skel" /></div>}
          {!loading && !candError && all.length === 0 && (source !== 'pair' || anchor) && <p className="fa-desk-hint" data-testid="pick-empty">{t(`pick.empty.${source}` as 'pick.empty.pair')}</p>}
          {!loading && !candError && all.length > 0 && cands.length === 0 && <p className="fa-desk-hint" data-testid="pick-filtered">{t('pick.filtered', { n: hiddenN })}</p>}
          <ul className="fa-desk-cands" role="listbox" aria-label={t('desk.picker')} data-stale={loading && all.length > 0 ? 'true' : undefined}>
            {cands.map((c, i) => {
              const inUse = same(current?.key, c.face); const pinned = pinnedKeys.has(keyStr(c.face)); const unavailable = c.row.missing;
              return (
              <li key={`${c.face.hashHex}:${c.face.faceIndex}`} role="option" aria-selected={i === idx} data-testid="desk-cand" data-trial={c.fromPeek ? 'peek' : undefined}
                onPointerEnter={() => setIdx(i)} onClick={() => { setIdx(i); }} onDoubleClick={() => { setIdx(i); void place(); }}>
                <FaceTitle api={api} face={c.row} className="fa-desk-cand-name">{nameOf(c.row)}</FaceTitle>
                {inUse && <span className="fa-desk-cand-sum" data-testid="cand-inuse">{t('pick.inUse')}</span>}
                {unavailable && <NoteMark inline text={t('pick.unavailable')} />}
                {c.row.supportsVi === false && c.row.viMissing > 0 && <NoteMark inline kind="warn" text={tn('strip.viMissing', { n: c.row.viMissing })} />}
                {c.summary && <Tooltip label={c.summary}><span className="fa-desk-cand-why" data-testid="cand-why">{c.summary}</span></Tooltip>}
                <Tooltip label={pinned ? t('pick.unpin') : t('pick.pin')} shortcut="P">
                  <button type="button" className="fa-desk-cand-pin" data-testid="desk-cand-pin" aria-pressed={pinned} aria-label={pinned ? t('pick.unpin') : t('pick.pin')}
                    onClick={e => { e.stopPropagation(); setIdx(i); void togglePin(c); }} onDoubleClick={e => e.stopPropagation()}>{pinned ? '★' : '☆'}</button>
                </Tooltip>
                {!inUse && !unavailable && (
                  <Tooltip label={t('desk.candAdd', { role: roleName(role) })} shortcut="+">
                    <button type="button" className="fa-desk-cand-add" data-testid="desk-cand-add" aria-label={t('desk.candAdd', { role: roleName(role) })}
                      onClick={e => { e.stopPropagation(); setIdx(i); void placeFace(c.face, c.row); }} onDoubleClick={e => e.stopPropagation()}>+</button>
                  </Tooltip>
                )}
              </li>);
            })}
          </ul>
          <p className="fa-desk-keys">{t('pick.keys')}</p>
        </section>
        <section className="fa-desk-page" aria-label={t('desk.sampleLabel')}>
          <div className="fa-desk-sheet">
            <div className="fa-desk-h" data-testid="desk-heading" style={{ ...css(headFam, headingFace), fontSize: 34, lineHeight: 1.15 }}>{sample.trim() && sample !== defaultSample() ? sample : t('desk.pageHeading')}</div>
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
      {confirmAll && data && (
        <Modal title={t('act.all.title', { n: data.faces.length })} onClose={() => setConfirmAll(false)}>
          <p>{t('act.all.body')}</p>
          <div className="fa-modal-actions">
            <button type="button" data-testid="desk-activate-cancel" onClick={() => setConfirmAll(false)}>{t('act.all.cancel')}</button>
            <button type="button" className="primary" data-testid="desk-activate-confirm" onClick={() => void runActivateAll()}>{t('act.all.ok')}</button>
          </div>
        </Modal>
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
