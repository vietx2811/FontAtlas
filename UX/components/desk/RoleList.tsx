import { useEffect, useRef, useState } from 'react';
import { beginFaceDrag, useFaceDragStore } from '../../store/faceDrag';
import type { FaceRow } from '../../features/library-grid/types';
import type { Api } from '../../api';
import { CmdError } from '../../api';
import type { ComboDto, ComboRoleDto } from '../../bindings';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { FaceTitle } from '../ui/FaceTitle';
import { NoteMark } from '../ui/noteMark';
import { Tooltip } from '../ui/Tooltip';
import { SUGGESTED_ROLES, presetKey, roleGloss, roleLabel } from './deskRoles';
import { faceLabel, useNameClashes } from '../../store/nameClash';
import { ClashNote } from '../ui/ClashNote';

const nameOf = (r: { faceId: number; family: string; subfamily: string | null } | null): string => (r ? faceLabel(r) : '—');
const say = (kind: 'info' | 'error', text: string, action?: { label: string; run(): void }) => void useNoticeStore.getState().push({ kind, text, action });

/** Tên người dùng gõ → tên lưu: trùng preset (kể cả tên đã dịch) thì lưu khoá preset để vi/en cùng đọc được. */
export const storedRoleName = (raw: string): string => presetKey(raw) ?? raw.trim();

function errText(e: unknown, name: string): string {
  const code = e instanceof CmdError ? e.code : '';
  if (code === 'role_name_invalid') return t('role.err.invalid');
  if (code === 'role_duplicate') return t('role.err.duplicate', { name });
  if (code === 'role_limit') return t('role.err.limit');
  if (code === 'role_not_found') return t('role.err.gone');
  if (code === 'role_set_mismatch') return t('role.err.mismatch');
  return t('role.err.other', { msg: e instanceof Error ? e.message : String(e) });
}

/** Cột vai của Pairing Desk (G3): chọn vai, + Role (tên tự do + gợi ý), đổi tên, đổi thứ tự, bỏ vai (có hoàn tác). */
export function RoleList({ api, combo, roles, role, onSelect, onCombo, onRenamed }: {
  api: Api; combo: ComboDto; /** Vai hiển thị theo thứ tự (gồm vai chỉ có ở giao diện khi Combo chưa lưu vai nào). */ roles: readonly ComboRoleDto[]; role: ComboRoleDto;
  onSelect(r: ComboRoleDto): void; onCombo(c: ComboDto, selectRole?: ComboRoleDto): void; onRenamed(from: ComboRoleDto, to: ComboRoleDto): void;
}): React.JSX.Element {
  useNameClashes();
  const drag = useFaceDragStore(d => d.drag);
  const [adding, setAdding] = useState(false);
  const [renaming, setRenaming] = useState<ComboRoleDto | null>(null);
  const [draft, setDraft] = useState('');
  const busy = useRef(false);
  const listRef = useRef<HTMLUListElement>(null);
  const [ins, setIns] = useState<number | null>(null); // vị trí chèn (0..n) trong các vai đã lưu khi đang kéo tay nắm
  const gripCleanup = useRef<(() => void) | null>(null);
  useEffect(() => () => gripCleanup.current?.(), []);
  const stored = combo.roles; // thứ tự đã lưu ở backend
  const have = new Set(roles.map(r => r.toLowerCase()));
  const suggestions = SUGGESTED_ROLES.filter(r => !have.has(r));
  const slotOf = (r: ComboRoleDto) => combo.slots.find(s => s.role === r);
  const inStore = (r: ComboRoleDto) => stored.includes(r);

  const run = async (name: string, f: () => Promise<void>): Promise<boolean> => {
    if (busy.current) return false;
    busy.current = true;
    try { await f(); return true; } catch (e) { say('error', errText(e, name)); return false; } finally { busy.current = false; }
  };
  const add = async (raw: string) => {
    const name = storedRoleName(raw);
    if (!name) return;
    const ok = await run(name, async () => {
      // Combo chưa lưu vai nào: các vai mặc định chỉ có ở giao diện; lưu chúng trước để thứ tự và vai mới khớp nhau.
      let c = combo;
      if (!c.roles.length) for (const r of roles) { if (!c.roles.some(x => x.toLowerCase() === r.toLowerCase())) c = await api.addComboRole(c.id, r); }
      c = await api.addComboRole(c.id, name);
      onCombo(c, c.roles.find(x => x.toLowerCase() === name.toLowerCase()) ?? name);
    });
    if (ok) { setAdding(false); setDraft(''); }
  };
  const remove = (r: ComboRoleDto) => {
    if (!inStore(r)) { onSelect(roles.find(x => x !== r) ?? r); return; } // vai chỉ có ở giao diện: không có gì để bỏ ở backend
    const before = [...stored]; const face = slotOf(r)?.key ?? null;
    void run(r, async () => {
      const c = await api.removeComboRole(combo.id, r);
      onCombo(c, role === r ? (c.roles[Math.max(0, before.indexOf(r) - 1)] ?? c.roles[0]) : undefined);
      say('info', t('role.removed', { role: roleLabel(r) }), {
        label: t('role.undo'),
        run: () => void run(r, async () => {
          let n = await api.addComboRole(combo.id, r);
          if (face) n = await api.setComboSlot(combo.id, r, face);
          n = await api.reorderComboRoles(combo.id, before.filter(x => n.roles.includes(x)));
          onCombo(n, r);
        }),
      });
    });
  };
  const move = (r: ComboRoleDto, d: -1 | 1) => {
    const i = stored.indexOf(r); const j = i + d;
    if (i < 0 || j < 0 || j >= stored.length) return;
    const next = [...stored]; [next[i], next[j]] = [next[j], next[i]];
    void run(r, async () => { onCombo(await api.reorderComboRoles(combo.id, next)); });
  };
  /** Kéo vai bằng tay nắm ở đầu hàng: thả vào khe giữa các vai (vạch chèn mảnh hiện vị trí) để đổi thứ tự; Esc huỷ; Hoàn tác trả thứ tự cũ. */
  const beginGrip = (e: React.PointerEvent, r: ComboRoleDto) => {
    if (e.button !== 0 || !inStore(r) || gripCleanup.current) return;
    e.preventDefault();
    const sx = e.clientX, sy = e.clientY; let moving = false; let at: number | null = null;
    const slots = () => [...(listRef.current?.querySelectorAll<HTMLElement>('[data-reorder-index]') ?? [])];
    const posOf = (y: number): number => { const els = slots(); let n = 0; for (const el of els) { const b = el.getBoundingClientRect(); if (y > b.top + b.height / 2) n++; } return n; };
    const end = () => {
      window.removeEventListener('pointermove', onMove, true); window.removeEventListener('pointerup', onUp, true); window.removeEventListener('pointercancel', abort, true);
      window.removeEventListener('keydown', onKey, true); window.removeEventListener('blur', abort);
      document.body.style.removeProperty('cursor'); setIns(null); gripCleanup.current = null;
    };
    const onMove = (ev: PointerEvent) => {
      if (!moving) { if (Math.hypot(ev.clientX - sx, ev.clientY - sy) < 4) return; moving = true; document.body.style.cursor = 'grabbing'; }
      at = posOf(ev.clientY); const i = stored.indexOf(r);
      setIns(at === i || at === i + 1 ? null : at); // thả ngay chỗ cũ: không có gì đổi
    };
    const onUp = () => {
      const to = at; const i = stored.indexOf(r); const was = moving; end();
      if (!was || to === null || to === i || to === i + 1) return;
      const before = [...stored]; const rest = before.filter(x => x !== r); const next = [...rest.slice(0, to > i ? to - 1 : to), r, ...rest.slice(to > i ? to - 1 : to)];
      void run(r, async () => {
        onCombo(await api.reorderComboRoles(combo.id, next));
        say('info', t('role.reordered', { role: roleLabel(r) }), { label: t('role.undo'), run: () => void run(r, async () => { onCombo(await api.reorderComboRoles(combo.id, before)); }) });
      });
    };
    const abort = () => end();
    const onKey = (ev: KeyboardEvent) => { if (ev.key === 'Escape') { ev.stopPropagation(); ev.preventDefault(); end(); } };
    gripCleanup.current = end;
    window.addEventListener('pointermove', onMove, true); window.addEventListener('pointerup', onUp, true); window.addEventListener('pointercancel', abort, true);
    window.addEventListener('keydown', onKey, true); window.addEventListener('blur', abort);
  };
  const rename = async (r: ComboRoleDto, raw: string) => {
    const name = storedRoleName(raw);
    if (!name || name === r) { setRenaming(null); return; }
    const ok = await run(name, async () => {
      const c = await api.renameComboRole(combo.id, r, name);
      onCombo(c); onRenamed(r, c.roles.find(x => x.toLowerCase() === name.toLowerCase()) ?? name);
    });
    if (ok) setRenaming(null);
  };

  return (
    <ul ref={listRef}>
      {roles.map(r => {
        const s = slotOf(r); const i = stored.indexOf(r); const gloss = roleGloss(r);
        return (
          <li key={r} className="fa-desk-roleitem" data-reorder-index={i >= 0 ? i : undefined} data-insert={ins !== null && i >= 0 ? (ins === i ? 'before' : ins === stored.length && i === stored.length - 1 ? 'after' : undefined) : undefined} data-drop-role={r} data-drop-name={roleLabel(r)} data-drop-here={drag?.fromRole === r ? 'true' : undefined}
            data-drop-armed={drag ? 'true' : undefined} data-drop-over={drag?.over?.kind === 'role' && drag.over.role === r ? (drag.over.valid ? 'ok' : 'no') : undefined}>
            {renaming !== r && inStore(r) && <Tooltip label={t('role.grip')}><span className="fa-desk-rolegrip" data-testid="role-grip" aria-hidden onPointerDown={e => beginGrip(e, r)}>⠿</span></Tooltip>}
            {renaming === r
              ? <input className="fa-desk-roleinput" data-testid="role-rename" aria-label={t('role.rename', { role: roleLabel(r) })} autoFocus defaultValue={roleLabel(r)} maxLength={40}
                  onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); void rename(r, e.currentTarget.value); } else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); setRenaming(null); } }}
                  onBlur={e => { if (renaming === r) void rename(r, e.currentTarget.value); }} />
              : <button type="button" className="fa-desk-role" data-testid="desk-role" data-role={r} aria-pressed={r === role} onClick={() => onSelect(r)}
                  onPointerDown={s && !s.missing ? e => beginFaceDrag(e, s.row as unknown as FaceRow, { solo: true, fromRole: r, ownButton: true }) : undefined}
                  onKeyDown={e => { if (e.key === 'F2') { e.preventDefault(); if (inStore(r)) setRenaming(r); } }}>
                  {gloss
                    ? <Tooltip label={gloss}><span className="fa-desk-role-name" data-term={`role.${presetKey(r)}`}>{roleLabel(r)}</span></Tooltip>
                    : <span className="fa-desk-role-name fa-desk-role-custom">{r}</span>}
                  {s
                    ? <><FaceTitle api={api} face={s.row} className="fa-desk-role-font">{nameOf(s.row)}</FaceTitle><ClashNote faceId={s.row.faceId} inline face={{ hashHex: s.row.hashHex, faceIndex: s.row.faceIndex }} /></>
                    : <span className="fa-desk-role-empty">{t('desk.roleEmpty')}</span>}
                  {s?.missing && <NoteMark inline text={t('desk.missing')} />}
                </button>}
            {renaming !== r && inStore(r) && (
              <span className="fa-desk-roleact" role="group" aria-label={roleLabel(r)}>
                <Tooltip label={t('role.up', { role: roleLabel(r) })}><button type="button" data-testid="role-up" aria-label={t('role.up', { role: roleLabel(r) })} disabled={i <= 0} onClick={() => move(r, -1)}>↑</button></Tooltip>
                <Tooltip label={t('role.down', { role: roleLabel(r) })}><button type="button" data-testid="role-down" aria-label={t('role.down', { role: roleLabel(r) })} disabled={i < 0 || i >= stored.length - 1} onClick={() => move(r, 1)}>↓</button></Tooltip>
                <Tooltip label={t('role.rename', { role: roleLabel(r) })} shortcut="F2"><button type="button" data-testid="role-rename-btn" aria-label={t('role.rename', { role: roleLabel(r) })} onClick={() => setRenaming(r)}>✎</button></Tooltip>
                <Tooltip label={t('role.removeTip')}><button type="button" data-testid="role-remove" aria-label={t('role.remove', { role: roleLabel(r) })} onClick={() => remove(r)}>×</button></Tooltip>
              </span>
            )}
          </li>
        );
      })}
      <li className="fa-desk-roleadd">
        {adding ? (
          <div className="fa-desk-addform">
            <input className="fa-desk-roleinput" data-testid="role-add-input" aria-label={t('role.name')} placeholder={t('role.name.ph')} autoFocus maxLength={40} value={draft}
              onChange={e => setDraft(e.target.value)}
              onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); void add(draft); } else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); setAdding(false); setDraft(''); } }} />
            {suggestions.length > 0 && (
              <div className="fa-desk-rolesug" role="group" aria-label={t('role.suggest')}>
                {suggestions.map(r => <button key={r} type="button" data-testid="role-suggest" data-role={r} onClick={() => void add(r)}>{roleLabel(r)}</button>)}
              </div>)}
          </div>
        ) : (
          <Tooltip label={t('role.add.tip')}><button type="button" className="fa-desk-addrole" data-testid="role-add" onClick={() => setAdding(true)}>{t('role.add')}</button></Tooltip>
        )}
      </li>
    </ul>
  );
}
