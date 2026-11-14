import { useCallback, useRef } from 'react';
import type { Api, ActivationReportDto, ConflictDto, FaceKeyDto, FileOutcomeDto, FileResultDto } from '../../api';
import { t } from '../../i18n';
import { refreshActivation } from '../../store/activationSync';
import { keepBoth } from './renamedCopy';
import { useActivationQueue } from '../../store/activationFlow';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { isSameFile, replaceAdvice, toItems, vfActionOf, vfPairOf, type ConflictItem } from './conflictModel';
import { runReplace } from './applyReplace';
import { describeOutcome } from './messages';

/** Báo các kết quả đáng báo (lỗi, bận…); thành công thì im lặng. */
export const sayOutcomes = (outcomes: FileOutcomeDto[]): void => outcomes.forEach(o => { const m = describeOutcome(o); if (m) useNoticeStore.getState().push(m); });

export type ActivationFlow = { toggle(faces: FaceKeyDto[], on: boolean): Promise<void>; activateCase(caseId: number): Promise<void>; activateCollection(collectionId: number): Promise<void>; setScope(faces: FaceKeyDto[], scope: 'session' | 'persistent'): Promise<void> };

/**
 * Bật/tắt kích hoạt cho một nhóm face. Hộp thoại (xung đột, đang được dự án giữ) KHÔNG nằm ở đây: chúng vào hàng đợi chung
 * (`useActivationQueue`) và `ActivationDialogHost` dựng đúng một hộp ở một chỗ, nên nhiều nơi gọi không chồng hộp thoại.
 */
export function useActivationFlow(api: Api): ActivationFlow {
  const push = useNoticeStore.getState().push;
  const refresh = useCallback(() => refreshActivation(api), [api]);

  /** Xử lý báo cáo "bật": font cùng phiên bản đã có → báo; xung đột thật → hàng đợi hộp thoại chung; còn lại nói thật qua thông báo. */
  const afterActivate = async (rep: ActivationReportDto, totalFonts: number) => {
    const asked = rep.results.filter(r => r.outcome.kind === 'needs_confirmation');
    const fresh = rep.results.filter(r => r.outcome.kind === 'already_available');
    if (fresh.length) push({ kind: 'info', text: t('act.conflict.same', { n: fresh.reduce((s, r) => s + Math.max(1, r.requestedFaces.length), 0) }) });
    // Chỉ những font khác phiên bản/khác file mới chờ trong hộp thoại (gộp theo family).
    const conflictsOf = (r: FileResultDto): ConflictDto[] => (r.outcome.kind === 'needs_confirmation' ? r.outcome.conflicts : []);
    const real = asked.filter(r => !isSameFile(conflictsOf(r))); // bảo hiểm: backend đã đổi mọi bản trùng hệt thành already_available
    if (real.length) {
      const keys = real.map(r => ({ hashHex: r.hashHex, faceIndex: r.requestedFaces[0] ?? 0 }));
      let rows: Awaited<ReturnType<Api['queryFaces']>>['items'] = [];
      try { rows = (await api.queryFaces({ faceKeys: keys, limit: keys.length })).items; } catch { /* không có tên: dùng tên PostScript */ }
      const items = toItems(rows, keys, real.map(conflictsOf));
      // G7: cặp VF↔static mà người dùng đã chọn "nhớ" → làm theo, chỉ khi cách đó được phép; không thì hỏi như thường
      if (totalFonts - fresh.length === keys.length && await applyRemembered(keys, items)) { sayOutcomes(rep.results.map(r => r.outcome)); return; }
      useActivationQueue.getState().enqueue({ type: 'conflict', total: totalFonts - fresh.length, conflictFaces: keys, items });
    }
    sayOutcomes(rep.results.map(r => r.outcome));
  };

  /** Lựa chọn đã nhớ cho cặp VF↔static (G7). Trả true nếu đã xử lý (không hiện hộp thoại). Không làm khi: chưa nhớ gì; không phải cặp
   *  thuần VF↔static; "dùng bản này" mà bản kia không thay được / do app lạ; "giữ cả hai" mà có font hệ thống (luôn được ưu tiên). */
  const applyRemembered = async (keys: FaceKeyDto[], items: ConflictItem[]): Promise<boolean> => {
    const pair = vfPairOf(items);
    if (!pair) return false;
    let policy: Awaited<ReturnType<Api['activationVfPolicy']>> = 'ask';
    try { policy = await api.activationVfPolicy(); } catch { return false; }
    if (policy === 'ask') return false;
    const action = vfActionOf(policy, pair.newIsVf);
    const kinds = { vf: t('act.vf.kind.vf'), static: t('act.vf.kind.static') };
    const [mine, other] = pair.newIsVf ? [kinds.vf, kinds.static] : [kinds.static, kinds.vf];
    if (action === 'keep') { push({ kind: 'info', text: t('act.vf.auto.keep', { kept: other, skipped: mine }) }); return true; }
    if (action === 'both') {
      if (items.some(i => i.conflicts.some(c => c.owner === 'system'))) return false;
      await keepBoth(api, keys);
      push({ kind: 'info', text: t('act.vf.auto.both') });
      return true;
    }
    const advice = replaceAdvice(items);
    if (advice.tier !== 'replace' || advice.unknownApp) return false;
    await runReplace(api, keys, items, faces => toggle(faces, true), t('act.vf.auto.use', { used: mine, old: other }));
    return true;
  };

  const toggle = useCallback(async (faces: FaceKeyDto[], on: boolean) => {
    try {
      if (on) {
        // Font đã "Có sẵn trên máy" đã thoả: không gọi lại (khỏi báo lại mỗi lần bấm).
        const { available } = useUiStore.getState();
        const todo = faces.filter(f => !available.has(f.hashHex));
        if (!todo.length) return;
        await afterActivate(await api.activationActivate(todo), new Set(faces.map(f => f.hashHex)).size);
      } else {
        const rep = await api.activationDeactivate(faces);
        const held = rep.results.flatMap(r => (r.outcome.kind === 'held_by_collections' ? r.outcome.collectionIds : []));
        if (held.length) useActivationQueue.getState().enqueue({ type: 'held', faces, collections: new Set(held).size });
        sayOutcomes(rep.results.map(r => r.outcome));
      }
    } catch (e) {
      push({ kind: 'error', text: t('act.fail', { msg: (e as Error).message }) });
    }
    await refresh();
  }, [api, refresh]); // eslint-disable-line react-hooks/exhaustive-deps

  /** Đang bật hàng loạt cho Case/Dự án nào: bấm đôi không gửi hai lần. */
  const busy = useRef(new Set<string>());
  const once = async (key: string, fn: () => Promise<void>) => { if (busy.current.has(key)) return; busy.current.add(key); try { await fn(); } finally { busy.current.delete(key); } };
  const uniq = (rep: ActivationReportDto) => new Set(rep.results.map(r => r.hashHex)).size;
  /** "Bật tất cả" của một Case: cùng luồng xung đột; font `missing` được nêu riêng, không tính là lỗi. */
  const activateCase = useCallback((caseId: number) => once(`case:${caseId}`, async () => {
    try {
      const r = await api.activationActivateCase(caseId);
      await afterActivate(r.report, uniq(r.report));
      if (r.skippedMissing.length) push({ kind: 'info', text: t('act.case.missing', { n: r.skippedMissing.length }) });
    } catch (e) { push({ kind: 'error', text: t('act.fail', { msg: (e as Error).message }) }); }
    await refresh();
  }), [api, refresh]); // eslint-disable-line react-hooks/exhaustive-deps
  /** "Bật tất cả" của một Dự án (collection). */
  const activateCollection = useCallback((collectionId: number) => once(`col:${collectionId}`, async () => {
    try { const rep = await api.activationActivateCollection(collectionId); await afterActivate(rep, uniq(rep)); }
    catch (e) { push({ kind: 'error', text: t('act.fail', { msg: (e as Error).message }) }); }
    await refresh();
  }), [api, refresh]); // eslint-disable-line react-hooks/exhaustive-deps

  /** Đổi phạm vi của font ĐANG bật (không tắt giữa chừng). */
  const setScope = useCallback(async (faces: FaceKeyDto[], scope: 'session' | 'persistent') => {
    try { sayOutcomes((await api.activationSetScope(faces, scope)).results.map(r => r.outcome).filter(o => o.kind === 'failed' || o.kind === 'busy')); }
    catch (e) { push({ kind: 'error', text: t('act.scope.fail', { msg: (e as Error).message }) }); }
    await refresh();
  }, [api, refresh]); // eslint-disable-line react-hooks/exhaustive-deps

  return { toggle, activateCase, activateCollection, setScope };
}
