import type { Api } from '../../api';
import { t } from '../../i18n';
import { refreshActivation } from '../../store/activationSync';
import { useActivationQueue } from '../../store/activationFlow';
import { useNoticeStore } from '../../store/notices';
import { ConflictDialog, HeldDialog } from './ActivationDialogs';
import { sayOutcomes, useActivationFlow } from './useActivationFlow';

/** Dựng đúng MỘT hộp thoại kích hoạt (đầu hàng đợi chung) cho cả app. Đặt một lần ở gốc, ngoài mọi vùng có thể bị gỡ khỏi cây. */
const APP_OF: Record<string, string> = { right_font: 'RightFont', font_base: 'FontBase', typeface: 'Typeface', adobe_fonts: 'Adobe Fonts', suitcase: 'Suitcase / Connect' };

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
      onConfirm={include => void run(async () => { if (include) sayOutcomes((await api.activationActivate(p.conflictFaces, { proceed: true })).results.map(r => r.outcome)); })}
      onReplace={() => void run(async () => {
        const rep = await api.activationReplace(p.conflictFaces);
        sayOutcomes(rep.results.map(r => r.outcome));
        const replaced = rep.results.flatMap(r => r.replaced ?? []);
        if (!replaced.length) return;
        const apps = [...new Set(replaced.map(x => APP_OF[x.owner] ?? t('act.replace.app.other')))].join(' / ');
        useNoticeStore.getState().push({
          kind: 'info', text: t('act.replace.done', { app: apps }),
          action: {
            label: t('act.replace.undo'),
            run: () => void (async () => {
              try {
                const outs = [];
                for (const x of replaced) outs.push(await api.activationRestoreReplaced(x.id));
                const missing = outs.includes('missing_file');
                useNoticeStore.getState().push({ kind: missing ? 'error' : 'info', text: t(missing ? 'act.replace.restoreMissing' : 'act.replace.restored') });
              } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('act.fail', { msg: (e as Error).message }) }); }
              await refreshActivation(api);
            })(),
          },
        });
      })}
      onRetry={() => void run(async () => { await flow.toggle(p.conflictFaces, true); })}
      onReveal={path => void api.revealLocation(path)} />;
  }
  return <HeldDialog key={p.faces.map(f => f.hashHex).join()} collectionCount={p.collections} onCancel={cancel}
    onConfirm={() => void run(async () => sayOutcomes((await api.activationDeactivate(p.faces, { force: true })).results.map(r => r.outcome)))} />;
}
