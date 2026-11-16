import { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { Api, FaceKeyDto } from '../../api';
import type { CaseDto, ComboDto, ComboSuggestionDto, FaceRowDto, RoleRefDto } from '../../bindings';
import { t, tn } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { defaultSample, useUiStore } from '../../store/ui';
import { useFontFamily } from '../detail/useFontFamily';
import { NoteMark } from '../ui/noteMark';
import { Tooltip } from '../ui/Tooltip';
import { ROLE_ORDER, isHeadingLike, presetKey, roleLabel } from './deskRoles';
import { guessProfile, whyText } from './pickerSources';
import { BetaBadge } from '../ui/BetaBadge';
import { INTENTS, intentName, matchIntent, type Intent } from './intents';
import { faceLabel, useNameClashes } from '../../store/nameClash';
import { ClashNote } from '../ui/ClashNote';

export const STYLE_IDS = ['contrast', 'superfamily', 'same_structure', 'same_era', 'display_vs_neutral', 'harmony'] as const;
const SUGGEST_DEBOUNCE_MS = 300;
const MAX_ROLES = 6;
const nameOf = (r: FaceRowDto) => faceLabel(r);
const kOf = (k: FaceKeyDto) => `${k.hashHex}:${k.faceIndex}`;
const say = (kind: 'info' | 'error', text: string, action?: { label: string; run(): void }) => void useNoticeStore.getState().push({ kind, text, action });
const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Vai gửi backend: khoá preset hoặc vai có hồ sơ đoán được; vai lạ bị bỏ (backend sẽ trả roles_invalid). */
function refOf(role: string): RoleRefDto | null {
  const k = presetKey(role) ?? guessProfile(role);
  return k ? { name: presetKey(role) ?? role, profile: presetKey(role) ? null : k, size: null } : null;
}

function Sample({ api, row, heading, text }: { api: Api; row: FaceRowDto; heading: boolean; text: string }): React.JSX.Element {
  const fam = useFontFamily(api, row);
  return <div className={heading ? 'fa-cs-h' : 'fa-cs-p'} style={{ fontFamily: fam ? `"${fam}", var(--font-ui)` : undefined, fontWeight: row.weightClass ?? undefined, fontStyle: row.isItalic ? 'italic' : undefined }}>{text}</div>;
}

type Ops = { refresh(): Promise<CaseDto>; go(id: number): void };

/** Ô tên combo + popover "Gợi ý combo" (G6). Tên đã gõ giữ nguyên khi đóng; mọi lần nhận có hoàn tác. */
export function ComboNameSuggest({ api, data, combo, ops, nameRef, open, onOpenChange, host, pinned, onUnpinAll }: { api: Api; data: CaseDto; combo: ComboDto; ops: Ops; nameRef: React.RefObject<HTMLInputElement | null>;
  /** Khung gợi ý mở = đang chọn "Combo" làm đối tượng gợi ý (Desk giữ trạng thái chọn). */ open: boolean; onOpenChange(o: boolean): void;
  /** Nơi vẽ khung gợi ý nội tuyến (cột giữa của Desk); không có thì vẽ ngay sau ô tên. */ host?: HTMLElement | null;
  /** Vai người dùng ghim ("Giữ font này khi gợi ý combo"): chỉ những vai này bị khoá. Mặc định không vai nào. */ pinned: readonly string[]; onUnpinAll(): void }): React.JSX.Element {
  useNameClashes();
  const sample = useUiStore(s => s.sample);
  const savedName = combo.name && combo.name !== `Combo ${combo.idx}` ? combo.name : '';
  const [draft, setDraft] = useState(savedName);
  const setOpen = onOpenChange;
  const [typed, setTyped] = useState(savedName); // đã debounce
  const [style, setStyle] = useState<string>(''); // '' = tất cả
  const [caseOnly, setCaseOnly] = useState(false);
  const [seed, setSeed] = useState(1);
  const [retry, setRetry] = useState(0);
  const [sugs, setSugs] = useState<ComboSuggestionDto[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [showSkel, setShowSkel] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [active, setActive] = useState(0);
  const pop = useRef<HTMLDivElement>(null);
  const skipFocusOpen = useRef(false);
  useEffect(() => { setDraft(savedName); setTyped(savedName); }, [combo.id, savedName]);
  useEffect(() => { const id = setTimeout(() => setTyped(draft), SUGGEST_DEBOUNCE_MS); return () => clearTimeout(id); }, [draft]);
  useEffect(() => { if (!loading) { setShowSkel(false); return; } const id = setTimeout(() => setShowSkel(true), 150); return () => clearTimeout(id); }, [loading]);

  const intent: Intent | null = useMemo(() => matchIntent(typed), [typed]);
  const slotRoles = combo.slots.map(s => s.role);
  const baseRoles: string[] = useMemo(() => {
    const r = intent ? [...intent.roles] : combo.roles.length ? [...combo.roles] : ['heading', 'body'];
    for (const s of slotRoles) if (!r.some(x => x.toLowerCase() === s.toLowerCase())) r.push(s);
    // Combo mới từ một font (chỉ một vai): gợi ý cần ít nhất một vai nữa để ghép, không thì chỉ lặp lại đúng font đó.
    if (r.length < 2) { const add = r.length && !isHeadingLike(r[0]) ? 'heading' : 'body'; if (!r.some(x => x.toLowerCase() === add)) r.push(add); }
    return r;
  }, [intent, combo.roles, slotRoles.join('|')]); // eslint-disable-line react-hooks/exhaustive-deps
  const refsAll = useMemo(() => baseRoles.map(refOf).filter((x): x is RoleRefDto => !!x).sort((a, b) => Number(isHeadingLike(b.name)) - Number(isHeadingLike(a.name))), [baseRoles]);
  // Backend nhận tối đa 6 vai: vai thừa bị cắt và có ghi chú (không để cả lệnh lỗi roles_invalid).
  const refs = useMemo(() => refsAll.slice(0, MAX_ROLES), [refsAll]);
  const cut = refsAll.slice(MAX_ROLES).map(r => r.name);
  const skipped = baseRoles.filter(r => !refOf(r));

  /** Thẻ giống hệt combo đang dùng (mọi vai cùng font) không phải "gợi ý". */
  const isCurrent = (x: ComboSuggestionDto) => x.slots.length > 0 && x.slots.every(sl => { const cur = combo.slots.find(c => c.role.toLowerCase() === sl.role.name.toLowerCase()); return !!cur && !cur.missing && kOf(cur.key) === kOf(sl.face); });
  const filledRoles = new Set(combo.slots.filter(s => !s.missing).map(s => s.role.toLowerCase()));
  const allPinned = refs.length > 0 && refs.every(r => pinned.some(p => (presetKey(p) ?? p).toLowerCase() === r.name.toLowerCase()) && [...filledRoles].some(f => (presetKey(f) ?? f).toLowerCase() === r.name.toLowerCase()));
  useEffect(() => {
    if (!open) return;
    let alive = true; setLoading(true); setErr(null);
    const pins = new Set(pinned.map(x => x.toLowerCase()));
    const locked0 = combo.slots.filter(s => !s.missing && pins.has(s.role.toLowerCase()) && refs.some(r => r.name.toLowerCase() === (presetKey(s.role) ?? s.role).toLowerCase())).map(s => ({ role: refs.find(r => r.name.toLowerCase() === (presetKey(s.role) ?? s.role).toLowerCase())!, face: s.key }));
    // Một font ở hai vai thì chỉ khoá lần đầu (backend báo locked_duplicate nếu khoá hai lần).
    const seenKey = new Set<string>(); const locked = locked0.filter(l => { const k = kOf(l.face); if (seenKey.has(k)) return false; seenKey.add(k); return true; });
    if (allPinned) { setSugs([]); setLoading(false); return; } // mọi vai đã ghim: không còn gì để gợi ý
    api.suggestCombos({ styles: style ? [style] : intent?.styles ? [...intent.styles] : [], roles: refs, locked, scope: caseOnly ? 'case' : 'library', caseId: caseOnly ? data.id : null, projectVi: !!data.settings.vi, k: 6, seed })
      .then(r => { if (alive) { setSugs(r.filter(x => !isCurrent(x))); setActive(0); setLoading(false); } }, e => { if (alive) { setSugs(null); setLoading(false); setErr(errMsg(e)); } });
    return () => { alive = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, api, data.id, combo.id, refs, style, pinned.join('|'), caseOnly, seed, retry, combo.slots.map(s => `${s.role}:${kOf(s.key)}`).join('|')]);

  const saveName = async () => {
    const n = draft.trim();
    if (n === savedName) return;
    try { await api.updateCombo(combo.id, { name: n || `Combo ${combo.idx}` }); await ops.refresh(); } catch (e) { say('error', t('desk.opFail', { msg: errMsg(e) })); }
  };

  const fill = async (target: number, s: ComboSuggestionDto) => { for (const sl of s.slots) await api.setComboSlot(target, sl.role.name, sl.face); };
  const takeAll = async (s: ComboSuggestionDto) => {
    try {
      const empty = combo.slots.length === 0;
      const base = draft.trim() || intentName(intent ?? INTENTS[0]).split(',')[0];
      if (empty) {
        await fill(combo.id, s); await ops.refresh();
        say('info', t('cs.done.fill', { name: combo.name || `Combo ${combo.idx}` }), { label: t('cs.undo'), run: () => void (async () => { try { for (const sl of s.slots) await api.setComboSlot(combo.id, sl.role.name, null); await ops.refresh(); say('info', t('cs.undone')); } catch (e) { say('error', t('cs.fail.take', { msg: errMsg(e) })); } })() });
      } else {
        const name = `${draft.trim() ? draft.trim() : base} 2`;
        const c = await api.createCombo(data.id, name);
        await fill(c.id, s); await ops.refresh(); ops.go(c.id);
        say('info', t('cs.done.all', { name: c.name }), { label: t('cs.undo'), run: () => void (async () => { try { await api.deleteCombo(c.id); await ops.refresh(); ops.go(combo.id); say('info', t('cs.undone')); } catch (e) { say('error', t('cs.fail.take', { msg: errMsg(e) })); } })() });
      }
      setOpen(false);
    } catch (e) { say('error', t('cs.fail.take', { msg: errMsg(e) })); }
  };
  const takeRole = async (s: ComboSuggestionDto, i: number) => {
    const sl = s.slots[i]; const prev = combo.slots.find(x => x.role.toLowerCase() === sl.role.name.toLowerCase())?.key ?? null;
    try {
      await api.setComboSlot(combo.id, sl.role.name, sl.face); await ops.refresh();
      say('info', t('cs.done.role', { font: nameOf(sl.row), role: roleLabel(sl.role.name) }), { label: t('cs.undo'), run: () => void (async () => { try { await api.setComboSlot(combo.id, sl.role.name, prev); await ops.refresh(); say('info', t('cs.undone')); } catch (e) { say('error', t('cs.fail.take', { msg: errMsg(e) })); } })() });
    } catch (e) { say('error', t('cs.fail.take', { msg: errMsg(e) })); }
  };

  const cols = 3;
  const move = (d: number) => setActive(i => Math.max(0, Math.min((sugs?.length ?? 1) - 1, i + d)));
  const onPopKey = (e: React.KeyboardEvent) => {
    const tgt = e.target as HTMLElement;
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); skipFocusOpen.current = true; setOpen(false); nameRef.current?.focus(); return; } // trả focus về ô tên mà không mở lại
    if (tgt.closest('input, select, textarea') ) return;
    if (e.key === 'ArrowRight') { e.preventDefault(); move(1); } else if (e.key === 'ArrowLeft') { e.preventDefault(); move(-1); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); move(cols); } else if (e.key === 'ArrowUp') { e.preventDefault(); move(-cols); }
    else if (e.key === 'Enter' && !tgt.closest('button') && sugs?.[active]) { e.preventDefault(); void takeAll(sugs[active]); }
    else if ((e.key === 'r' || e.key === 'R') && !e.metaKey && !e.ctrlKey) { e.preventDefault(); setSeed(n => n + 1); }
  };
  const shownName = draft.trim() || '';
  const headText = sample.trim() && sample !== defaultSample() ? sample : t('desk.pageHeading');
  const bodyText = t('desk.pageBody').slice(0, 70);

  const panel = !open ? null : (
        <div ref={pop} className={`fa-cs${host ? ' inline' : ''}`} role="region" aria-label={t('cs.dialog')} data-testid="combo-suggest" onKeyDown={onPopKey}>
          <header>
            <b data-testid="cs-title">{t('desk.suggestFor', { target: t('desk.target.combo', { n: combo.idx }) })}{shownName ? ` · ${shownName}` : ''}<BetaBadge /></b>
            <label>{t('cs.style')} <select data-testid="cs-style" value={style} onChange={e => setStyle(e.target.value)}>
              <option value="">{t('cs.style.all')}</option>{STYLE_IDS.map(s => <option key={s} value={s}>{t(`style.${s}` as 'style.contrast')}</option>)}</select></label>
            {pinned.length > 0 && <span className="fa-cs-pins" data-testid="cs-pins">{tn('cs.pins', { n: pinned.length })} <button type="button" data-testid="cs-unpin-all" onClick={onUnpinAll}>{t('cs.unpinAll')}</button></span>}
            <label><input type="checkbox" data-testid="cs-caseonly" checked={caseOnly} onChange={e => setCaseOnly(e.target.checked)} /> {t('cs.libOnly')}</label>
            <button type="button" data-testid="cs-again" onClick={() => setSeed(n => n + 1)}>⟳ {t('cs.again')}</button>
          </header>
          <p className="fa-cs-intent" data-testid="cs-intent">{intent ? t('cs.understood', { intent: intentName(intent) }) : t('cs.unknown')}{skipped.length > 0 && <NoteMark inline kind="info" text={t('cs.roles.custom', { roles: skipped.join(', ') })} />}{cut.length > 0 && <NoteMark inline kind="warn" testId="cs-cut" text={t('cs.roles.cut', { n: MAX_ROLES, roles: cut.join(', ') })} />}</p>
          <span className="fa-cs-live" role="status" aria-live="polite">{sugs && !loading ? t('cs.count', { n: sugs.length }) : ''}</span>
          {err && <div role="alert" className="fa-desk-err">{t('cs.fail', { msg: err })} <button type="button" data-testid="cs-retry" onClick={() => setRetry(n => n + 1)}>{t('cs.retry')}</button></div>}
          {loading && showSkel && !sugs && <p className="fa-desk-hint" data-testid="cs-loading" aria-busy="true">{t('cs.loading')}</p>}
          {allPinned && <div className="fa-desk-hint" data-testid="cs-allpinned">{t('cs.allPinned')} <button type="button" data-testid="cs-unpin-all2" onClick={onUnpinAll}>{t('cs.unpinAll')}</button></div>}
          {!allPinned && !loading && !err && sugs && sugs.length === 0 && (
            <div className="fa-desk-hint" data-testid="cs-empty">{t('cs.empty')} {pinned.length > 0 && <button type="button" onClick={onUnpinAll}>{t('cs.unpinAll')}</button>} {style && <button type="button" onClick={() => setStyle('')}>{t('cs.empty.styles')}</button>}</div>
          )}
          <div className="fa-cs-grid" role="listbox" aria-label={t('cs.dialog')} data-stale={loading && sugs ? 'true' : undefined}>
            {(sugs ?? []).map((s, i) => (
              <div key={i} className="fa-cs-card" role="option" tabIndex={i === active ? 0 : -1} aria-selected={i === active} data-testid="cs-card" onFocus={() => setActive(i)} onClick={() => setActive(i)} onDoubleClick={() => void takeAll(s)}
                onKeyDown={e => { if (e.key === 'Enter' && e.target === e.currentTarget) { e.preventDefault(); e.stopPropagation(); void takeAll(s); } }}>
                {s.slots.map((sl, k) => (
                  <div key={sl.role.name} className="fa-cs-slot">
                    <Sample api={api} row={sl.row} heading={isHeadingLike(sl.role.name) && k === 0} text={isHeadingLike(sl.role.name) ? headText : bodyText} />
                    <div className="fa-cs-meta"><span>{roleLabel(sl.role.name)} · {nameOf(sl.row)}<ClashNote faceId={sl.row.faceId} inline />{sl.locked ? ' 🔒' : ''}</span>
                      {!sl.locked && <Tooltip label={t('cs.takeRole', { role: roleLabel(sl.role.name) })}><button type="button" data-testid="cs-take-role" aria-label={t('cs.takeRole', { role: roleLabel(sl.role.name) })} onClick={e => { e.stopPropagation(); void takeRole(s, k); }}>＋</button></Tooltip>}</div>
                  </div>
                ))}
                <div className="fa-cs-why">{s.style ? t(`style.${s.style}` as 'style.contrast') : ''}{s.reasons.map(whyText).filter(Boolean).slice(0, 2).map(x => ` · ${x}`).join('')}
                  {' · '}{s.viMissing === 0 ? t('cs.vi.ok') : <>{t('cs.vi.miss', { n: s.viMissing })} <NoteMark inline kind="warn" text={tn('strip.viMissing', { n: s.viMissing })} /></>}</div>
                <button type="button" className="fa-cs-take" data-testid="cs-take-all" onClick={e => { e.stopPropagation(); void takeAll(s); }}>{t('cs.takeAll')}</button>
              </div>
            ))}
          </div>
          <p className="fa-desk-keys">{t('cs.keys')}</p>
        </div>
  );

  return (
    <span className="fa-comboname">
      <input ref={nameRef} className="fa-comboname-input" data-testid="combo-name" value={draft} placeholder={t('cs.name.ph')} aria-label={t('cs.name.label')} maxLength={120} aria-expanded={open}
        onChange={e => { setDraft(e.target.value); setOpen(true); }} onFocus={() => { if (skipFocusOpen.current) { skipFocusOpen.current = false; return; } setOpen(true); }} onClick={() => setOpen(true)}
        onKeyDown={e => {
          if (e.key === 'Enter') { e.preventDefault(); void saveName(); }
          else if (e.key === 'Escape') { e.stopPropagation(); if (open) setOpen(false); else { setDraft(savedName); (e.currentTarget as HTMLInputElement).blur(); } }
          else if (e.key === 'ArrowDown') { e.preventDefault(); if (!open) setOpen(true); else pop.current?.querySelector<HTMLElement>('[data-testid=cs-card]')?.focus(); }
        }}
        onBlur={() => { void saveName(); }} />
      {host ? (panel ? createPortal(panel, host) : null) : panel}
    </span>
  );
}
