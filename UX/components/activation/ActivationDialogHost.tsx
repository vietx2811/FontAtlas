import type { Api, FaceKeyDto } from '../../api';
import { t } from '../../i18n';
import { refreshActivation } from '../../store/activationSync';
import { useActivationQueue } from '../../store/activationFlow';
import { useNoticeStore } from '../../store/notices';
import { ConflictDialog, HeldDialog } from './ActivationDialogs';
import { runReplace } from './applyReplace';
import { vfPairOf } from './conflictModel';
import { copyDoneText, deriveErrorText, makeRenamedCopy } from './renamedCopy';
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
      onConfirm={include => void run(async () => {
        if (!include) return;
        // G7b: "Giữ cả hai" với font VF sắp bật → tạo bản sao đổi tên (họ "X VF") rồi bật BẢN SAO cùng bản static; hiện tên mới.
        if (vfPairOf(p.items)?.newIsVf) {
          const copies: FaceKeyDto[] = [];
          for (const f of p.conflictFaces) {
            try {
              const { copy, name } = await makeRenamedCopy(api, f);
              copies.push(copy.face);
              useNoticeStore.getState().push({ kind: 'info', text: copyDoneText(copy, name) });
            } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: deriveErrorText(e) }); }
          }
          if (copies.length) sayOutcomes((await api.activationActivate(copies)).results.map(r => r.outcome));
          return;
        }
        sayOutcomes((await api.activationActivate(p.conflictFaces, { proceed: true })).results.map(r => r.outcome));
      })}
      onReplace={() => void run(() => runReplace(api, p.conflictFaces, p.items, faces => flow.toggle(faces, true)))}
      onRemember={choice => void api.activationSetVfPolicy(choice).catch(() => undefined)}
      onRetry={() => void run(async () => { await flow.toggle(p.conflictFaces, true); })}
      onReveal={path => void api.revealLocation(path)} />;
  }
  return <HeldDialog key={p.faces.map(f => f.hashHex).join()} collectionCount={p.collections} onCancel={cancel}
    onConfirm={() => void run(async () => sayOutcomes((await api.activationDeactivate(p.faces, { force: true })).results.map(r => r.outcome)))} />;
}
