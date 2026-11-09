import { useCallback, useState } from 'react';
import type { Api, ConflictDto, FaceKeyDto, FileOutcomeDto, FileResultDto } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { ConflictDialog, HeldDialog } from './ActivationDialogs';
import { isSameFile, toItems, type ConflictItem } from './conflictModel';
import { describeOutcome } from './messages';

type Pending = { type: 'conflict'; total: number; conflictFaces: FaceKeyDto[]; items: ConflictItem[] } | { type: 'held'; faces: FaceKeyDto[]; collections: number };

/** Bật/tắt kích hoạt cho một nhóm face kèm các hộp thoại D3 và "đang được giữ bởi dự án". */
export function useActivationFlow(api: Api): { toggle(faces: FaceKeyDto[], on: boolean): Promise<void>; setScope(faces: FaceKeyDto[], scope: 'session' | 'persistent'): Promise<void>; dialog: React.ReactNode } {
  const [pending, setPending] = useState<Pending | null>(null);
  const push = useNoticeStore.getState().push;
  const refresh = useCallback(() => useUiStore.getState().refreshActive(api), [api]);

  const say = (outcomes: FileOutcomeDto[]) => outcomes.forEach(o => { const m = describeOutcome(o); if (m) push(m); });

  const toggle = useCallback(async (faces: FaceKeyDto[], on: boolean) => {
    try {
      if (on) {
        const rep = await api.activationActivate(faces);
        const asked = rep.results.filter(r => r.outcome.kind === 'needs_confirmation');
        // Font không trùng đã được bật ngay. Font mà bản đang cài là CHÍNH file này (sameFile) đã dùng được: không hỏi, chỉ báo một dòng.
        // Chỉ những font khác phiên bản/khác file mới chờ trong hộp thoại (gộp theo family).
        const conflictsOf = (r: FileResultDto): ConflictDto[] => (r.outcome.kind === 'needs_confirmation' ? r.outcome.conflicts : []);
        const same = asked.filter(r => isSameFile(conflictsOf(r)));
        const real = asked.filter(r => !same.includes(r));
        if (same.length) push({ kind: 'info', text: t('act.conflict.same', { n: same.reduce((s, r) => s + Math.max(1, r.requestedFaces.length), 0) }) });
        if (real.length) {
          const keys = real.map(r => ({ hashHex: r.hashHex, faceIndex: r.requestedFaces[0] ?? 0 }));
          let rows: Awaited<ReturnType<Api['queryFaces']>>['items'] = [];
          try { rows = (await api.queryFaces({ faceKeys: keys, limit: keys.length })).items; } catch { /* không có tên: dùng tên PostScript */ }
          setPending({ type: 'conflict', total: new Set(faces.map(f => f.hashHex)).size - same.length, conflictFaces: keys, items: toItems(rows, keys, real.map(conflictsOf)) });
        }
        say(rep.results.map(r => r.outcome));
      } else {
        const rep = await api.activationDeactivate(faces);
        const held = rep.results.flatMap(r => (r.outcome.kind === 'held_by_collections' ? r.outcome.collectionIds : []));
        if (held.length) setPending({ type: 'held', faces, collections: new Set(held).size });
        say(rep.results.map(r => r.outcome));
      }
    } catch (e) {
      push({ kind: 'error', text: `Không đổi được kích hoạt: ${(e as Error).message}` });
    }
    await refresh();
  }, [api, refresh]); // eslint-disable-line react-hooks/exhaustive-deps

  /** Đổi phạm vi của font ĐANG bật (không tắt giữa chừng). */
  const setScope = useCallback(async (faces: FaceKeyDto[], scope: 'session' | 'persistent') => {
    try { say((await api.activationSetScope(faces, scope)).results.map(r => r.outcome).filter(o => o.kind === 'failed' || o.kind === 'busy')); }
    catch (e) { push({ kind: 'error', text: t('act.scope.fail', { msg: (e as Error).message }) }); }
    await refresh();
  }, [api, refresh]); // eslint-disable-line react-hooks/exhaustive-deps

  const run = async (fn: () => Promise<unknown>) => {
    setPending(null);
    try { await fn(); } catch (e) { push({ kind: 'error', text: `Không đổi được kích hoạt: ${(e as Error).message}` }); }
    await refresh();
  };

  let dialog: React.ReactNode = null;
  if (pending?.type === 'conflict') {
    const p = pending;
    dialog = <ConflictDialog items={p.items} total={p.total} onCancel={() => setPending(null)}
      onConfirm={include => void run(async () => { if (include) say((await api.activationActivate(p.conflictFaces, { proceed: true })).results.map(r => r.outcome)); })} />;
  } else if (pending?.type === 'held') {
    const p = pending;
    dialog = <HeldDialog collectionCount={p.collections} onCancel={() => setPending(null)}
      onConfirm={() => void run(async () => say((await api.activationDeactivate(p.faces, { force: true })).results.map(r => r.outcome)))} />;
  }
  return { toggle, setScope, dialog };
}
