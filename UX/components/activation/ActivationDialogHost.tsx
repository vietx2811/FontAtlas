import type { Api } from '../../api';
import { t } from '../../i18n';
import { refreshActivation } from '../../store/activationSync';
import { useActivationQueue } from '../../store/activationFlow';
import { useNoticeStore } from '../../store/notices';
import { ConflictDialog, HeldDialog } from './ActivationDialogs';
import { runReplace } from './applyReplace';
import { keepBoth } from './renamedCopy';
import { sayOutcomes, useActivationFlow } from './useActivationFlow';

/** Dựng đúng MỘT hộp thoại kích hoạt (đầu hàng đợi chung) cho cả app. Đặt một lần ở gốc, ngoài mọi vùng có thể bị gỡ khỏi cây. */
export function ActivationDialogHost({ api }: { api: Api }): React.JSX.Element | null {
  const p = useActivationQueue(s => s.queue[0]);
  const flow = useActivationFlow(api);
  if (!p) return null;
  const run = async (fn: () => Promise<unknown>) => {
    useActivationQueue.getState().shift();
    try { await fn(); } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('act.fail', { msg: (e as Error).message }) }); }
    await refreshActivation(api);
  };
  const cancel = () => useActivationQueue.getState().shift();
  if (p.type === 'conflict') {
    return <ConflictDialog key={p.conflictFaces.map(f => f.hashHex).join()} items={p.items} total={p.total} onCancel={cancel}
      onConfirm={include => void run(async () => { if (include) await keepBoth(api, p.conflictFaces); })}
      onReplace={() => void run(() => runReplace(api, p.conflictFaces, p.items, faces => flow.toggle(faces, true)))}
      onRemember={choice => void api.activationSetVfPolicy(choice).catch(() => undefined)}
      onRetry={() => void run(async () => { await flow.toggle(p.conflictFaces, true); })}
      onReveal={path => void api.revealLocation(path)} />;
  }
  return <HeldDialog key={p.faces.map(f => f.hashHex).join()} collectionCount={p.collections} onCancel={cancel}
    onConfirm={() => void run(async () => sayOutcomes((await api.activationDeactivate(p.faces, { force: true })).results.map(r => r.outcome)))} />;
}
