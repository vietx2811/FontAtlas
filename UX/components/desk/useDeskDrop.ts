import { useEffect, useRef } from 'react';
import type { Api } from '../../api';
import type { ComboDto, ComboRoleDto } from '../../bindings';
import type { FaceRow } from '../../features/library-grid/types';
import { t } from '../../i18n';
import { registerFaceDropHandler } from '../../store/faceDrag';
import { faceLabel } from '../../store/nameClash';
import { useNoticeStore } from '../../store/notices';
import { dropFacesOn } from '../dnd/dropFaces';
import { useFaceActions } from '../faces/FaceActions';
import { roleLabel } from './deskRoles';

const say = (kind: 'info' | 'error', text: string, action?: { label: string; run(): void }): void => void useNoticeStore.getState().push({ kind, text, action });
const same = (a: { hashHex: string; faceIndex: number } | null | undefined, b: { hashHex: string; faceIndex: number } | null | undefined): boolean => !!a && !!b && a.hashHex === b.hashHex && a.faceIndex === b.faceIndex;

/**
 * G8: nơi nhận lượt thả khi Pairing Desk đang mở (Desk phủ lên Library nên đứng trên cùng ngăn xếp).
 * Thả ứng viên lên một vai = đặt font vào vai đó; thả một vai lên vai khác = đổi chỗ hai font. Mọi thao tác có Hoàn tác.
 * Đích khác (Dự án, Case, Đang bật, thẻ, So sánh) chuyển cho bộ xử lý chung.
 */
export function useDeskDrop(api: Api, combo: ComboDto | null, onCombo: (c: ComboDto) => void): void {
  const actions = useFaceActions();
  const live = useRef({ combo, onCombo, actions });
  live.current = { combo, onCombo, actions };
  useEffect(() => registerFaceDropHandler((rows, target, opts) => {
    const { combo: c, onCombo: apply, actions: act } = live.current;
    if (target.kind !== 'role') { void dropFacesOn(api, rows, target, { setActive: act?.setActive }, { moveFrom: null }); return; }
    if (!c) return;
    const role: ComboRoleDto = target.role ?? '';
    const slotOf = (r: ComboRoleDto) => c.slots.find(s => s.role === r);
    const fail = (r: string, e: unknown) => say('error', t('dnd.roleFail', { role: roleLabel(r), msg: e instanceof Error ? e.message : String(e) }));
    const set = async (r: ComboRoleDto, key: { hashHex: string; faceIndex: number } | null) => apply(await api.setComboSlot(c.id, r, key));
    if (opts.fromRole) {
      const from = opts.fromRole; const a = slotOf(from); const b = slotOf(role);
      if (from === role || !a) return;
      void (async () => {
        try {
          await set(role, a.key); await set(from, b?.key ?? null);
          say('info', t('dnd.roleSwapped', { a: roleLabel(from), b: roleLabel(role) }), {
            label: t('dnd.undo'),
            run: () => void (async () => { try { await set(from, a.key); await set(role, b?.key ?? null); say('info', t('dnd.roleUndone', { role: roleLabel(role) })); } catch (e) { fail(role, e); } })(),
          });
        } catch (e) { fail(role, e); }
      })();
      return;
    }
    const row = rows[0] as FaceRow | undefined; if (!row) return;
    const key = { hashHex: row.hashHex, faceIndex: row.faceIndex }; const prev = slotOf(role);
    if (same(prev?.key, key)) { say('info', t('dnd.roleSame', { font: faceLabel(row), role: roleLabel(role) })); return; }
    void (async () => {
      try {
        await set(role, key);
        say('info', t('dnd.rolePlaced', { font: faceLabel(row), role: roleLabel(role) }), {
          label: t('dnd.undo'),
          run: () => void (async () => { try { await set(role, prev?.key ?? null); say('info', t('dnd.roleUndone', { role: roleLabel(role) })); } catch (e) { fail(role, e); } })(),
        });
      } catch (e) { fail(role, e); }
    })();
  }), [api]);
}
