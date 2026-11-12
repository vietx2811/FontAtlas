import type { Api } from '../../api';
import type { FaceRow } from '../../features/library-grid/types';
import { formatNumber, t, tn } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { rowKey } from '../../store/selection';
import { useUiStore } from '../../store/ui';
import { faceName } from '../faces/faceMenuModel';
import type { DropTarget } from '../../store/faceDrag';

const keyOf = (r: FaceRow) => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });
const what = (rs: readonly FaceRow[]): string => (rs.length === 1 ? `«${faceName(rs[0])}»` : tn('faces.nFonts', { n: formatNumber(rs.length) }));
const say = (kind: 'info' | 'error', text: string, action?: { label: string; run(): void }): void => void useNoticeStore.getState().push({ kind, text, action });

/**
 * Thả font vào một dự án (G1): chỉ thêm font chưa có, báo kết quả; thông báo có "Hoàn tác" chỉ gỡ đúng phần vừa thêm
 * (font đã có sẵn từ trước không bị gỡ).
 */
export async function dropFacesIntoCollection(api: Api, rows: readonly FaceRow[], target: Pick<DropTarget, 'id' | 'name'>): Promise<void> {
  try {
    const there = await api.queryFaces({ collectionId: target.id, faceKeys: rows.map(keyOf), limit: rows.length });
    const have = new Set(there.items.map(r => rowKey(r)));
    const fresh = rows.filter(r => !have.has(rowKey(r)));
    if (!fresh.length) { say('info', t('dnd.allThere', { name: target.name })); return; }
    await api.addToCollection(target.id, fresh.map(keyOf));
    useUiStore.getState().bumpLibrary(false);
    const undo = {
      label: t('dnd.undo'),
      run: () => {
        api.removeFromCollection(target.id, fresh.map(keyOf)).then(
          () => { say('info', t('dnd.undone', { what: what(fresh), name: target.name })); useUiStore.getState().bumpLibrary(useUiStore.getState().view === `col:${target.id}`); },
          e => say('error', t('dnd.undoFail', { msg: (e as Error)?.message ?? String(e) })),
        );
      },
    };
    say('info', fresh.length === rows.length ? t('faces.addedTo', { what: what(rows), name: target.name }) : t('dnd.addedSome', { what: what(fresh), name: target.name, k: rows.length - fresh.length }), undo);
  } catch (e) { say('error', t('dnd.addFail', { name: target.name, msg: (e as Error)?.message ?? String(e) })); }
}
