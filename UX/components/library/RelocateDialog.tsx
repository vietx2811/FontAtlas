import { useState } from 'react';
import type { Api, RelocateTargetDto } from '../../api';
import { formatNumber, t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { Modal } from '../Modal';
import { busyFaces, busyPart } from '../activation/busyNotice';
import './organized.css';

/** Kết quả theo file → số liệu cho thông báo: chuyển được / đã ở bản mới / không đổi được (đường cũ vẫn dùng). */
export function relocateCounts(results: ReadonlyArray<{ outcome: { kind: string } }>): { moved: number; same: number; kept: number } {
  let moved = 0, same = 0, kept = 0;
  for (const r of results) {
    const k = r.outcome.kind;
    if (k === 'activated') moved++; else if (k === 'already_active') same++; else if (k === 'missing_file' || k === 'not_downloaded' || k === 'failed') kept++;
  }
  return { moved, same, kept };
}

/** Hoàn tác: chuyển ngược về đường cũ đã ghi; font nào không chuyển ngược được thì báo số lượng (đường hiện tại vẫn dùng). */
export async function undoRelocate(api: Api, back: RelocateTargetDto[]): Promise<void> {
  try {
    const rep = await api.activationRelocateTo(back);
    await useUiStore.getState().refreshActive(api);
    const c = relocateCounts(rep.results);
    useNoticeStore.getState().push({ kind: c.kept > 0 ? 'error' : 'info', text: t('org.reloc.undone', { n: formatNumber(c.moved + c.same) }) + (c.kept > 0 ? ` ${t('org.reloc.undoKept', { k: formatNumber(c.kept) })}` : '') });
  } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('dnd.undoFail', { msg: (e as Error)?.message ?? String(e) }) }); }
}

/** "Chuyển font đang bật sang bản trong thư viện" (chỉ khi người dùng bấm): đăng ký đường mới trước, gỡ đường cũ sau; lỗi thì đường cũ vẫn dùng. */
export function RelocateDialog({ api, preferUnder, onClose }: { api: Api; preferUnder: string | null; onClose(): void }): React.JSX.Element {
  const active = useUiStore(s => s.active);
  const [busy, setBusy] = useState(false);
  const faces = [...active.entries()].flatMap(([hashHex, idx]) => idx.map(faceIndex => ({ hashHex, faceIndex })));
  const run = async () => {
    setBusy(true);
    try {
      // Ghi đường đang đăng ký TRƯỚC khi chuyển: Hoàn tác chuyển ngược đúng từng font về đó (bản gốc không bị đụng nên chuyển ngược được).
      const before = new Map((await api.activationState()).filter(x => x.state === 'active' && x.activePath).map(x => [x.hashHex, x.activePath as string] as const));
      /** MỘT thông báo cho mỗi lượt: kết quả + "N đang bận" + [Hoàn tác] và/hoặc [Thử lại] (chạy lại relocate cho đúng các font bận, báo lại qua cùng hàm). */
      const report = async (rep: Awaited<ReturnType<Api['activationRelocate']>>, ofFaces: typeof faces, retried: boolean): Promise<void> => {
        await useUiStore.getState().refreshActive(api);
        const c = relocateCounts(rep.results);
        const busy = busyPart(rep);
        const parts: string[] = [];
        if (c.moved + c.same + c.kept > 0 || !retried) parts.push(t('org.reloc.done', { n: formatNumber(c.moved), same: formatNumber(c.same) }));
        if (c.kept > 0) parts.push(t('org.reloc.kept', { k: formatNumber(c.kept) }));
        if (busy) parts.push(busy);
        if (parts.length === 0) parts.push(t('act.busy.cleared'));
        const movedHashes = new Set(rep.results.filter(r => r.outcome.kind === 'activated').map(r => r.hashHex));
        const back: RelocateTargetDto[] = ofFaces.filter(f => movedHashes.has(f.hashHex) && before.has(f.hashHex)).map(f => ({ face: f, path: before.get(f.hashHex)! }));
        const retry = busy ? { label: t('act.busy.retry'), run: () => { const bf = busyFaces(rep); api.activationRelocate(bf, preferUnder).then(r => report(r, bf, true), e => useNoticeStore.getState().push({ kind: 'error', text: t('org.err.other', { msg: (e as Error)?.message ?? String(e) }) })); } } : undefined;
        const undo = back.length ? { label: t('dnd.undo'), run: () => void undoRelocate(api, back) } : undefined;
        useNoticeStore.getState().push({ kind: c.kept > 0 ? 'error' : 'info', text: parts.join(' '), action: undo ?? retry, action2: undo ? retry : undefined });
      };
      const rep = await api.activationRelocate(faces, preferUnder);
      onClose();
      await report(rep, faces, false);
    } catch (e) { onClose(); useNoticeStore.getState().push({ kind: 'error', text: t('org.err.other', { msg: (e as Error)?.message ?? String(e) }) }); }
  };
  return (
    <Modal title={t('org.reloc.title')} onClose={onClose} className="fa-org-dialog">
      <p data-testid="org-reloc-body">{faces.length === 0 ? t('org.reloc.none') : t('org.reloc.body', { n: formatNumber(faces.length) })}</p>
      <div className="fa-modal-actions">
        <button type="button" onClick={onClose}>{t('org.cancel')}</button>
        <button type="button" className="primary" data-testid="org-reloc-run" disabled={busy || faces.length === 0} onClick={() => void run()}>{t('org.reloc.run')}</button>
      </div>
    </Modal>
  );
}
