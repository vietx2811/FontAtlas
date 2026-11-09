import { useCallback, useState } from 'react';
import type { Api, ConflictDto, FaceKeyDto, FileOutcomeDto } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { ConflictDialog, HeldDialog } from './ActivationDialogs';
import { describeOutcome } from './messages';

type Pending = { type: 'conflict'; total: number; conflictFaces: FaceKeyDto[]; conflicts: ConflictDto[] } | { type: 'held'; faces: FaceKeyDto[]; collections: number };

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
        // Font không trùng đã được bật ngay; chỉ những font cần xác nhận mới chờ trong hộp thoại.
        if (asked.length) {
          setPending({
            type: 'conflict', total: new Set(faces.map(f => f.hashHex)).size,
            conflictFaces: asked.map(r => ({ hashHex: r.hashHex, faceIndex: r.requestedFaces[0] ?? 0 })),
            conflicts: asked.flatMap(r => (r.outcome.kind === 'needs_confirmation' ? r.outcome.conflicts : [])),
          });
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
    dialog = <ConflictDialog conflicts={p.conflicts} total={p.total} onCancel={() => setPending(null)}
      onConfirm={include => void run(async () => { if (include) say((await api.activationActivate(p.conflictFaces, { proceed: true })).results.map(r => r.outcome)); })} />;
  } else if (pending?.type === 'held') {
    const p = pending;
    dialog = <HeldDialog collectionCount={p.collections} onCancel={() => setPending(null)}
      onConfirm={() => void run(async () => say((await api.activationDeactivate(p.faces, { force: true })).results.map(r => r.outcome)))} />;
  }
  return { toggle, setScope, dialog };
}
