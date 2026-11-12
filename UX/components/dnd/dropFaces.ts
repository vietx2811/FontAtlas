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
 * Thả font vào một dự án hoặc một Case (G1): chỉ thêm font chưa có, báo kết quả; thông báo có "Hoàn tác" chỉ gỡ đúng phần
 * vừa thêm (font đã có sẵn từ trước không bị gỡ).
 */
export async function dropFacesInto(api: Api, rows: readonly FaceRow[], target: Pick<DropTarget, 'kind' | 'id' | 'name'>): Promise<void> {
  const isCase = target.kind === 'case';
  const name = isCase ? `Case ${target.name}` : target.name; // "Case" là thuật ngữ giữ nguyên ở mọi ngôn ngữ
  try {
    const have = new Set<string>();
    if (isCase) (await api.getCase(target.id)).faces.forEach(f => have.add(`${f.key.hashHex}:${f.key.faceIndex}`));
    else (await api.queryFaces({ collectionId: target.id, faceKeys: rows.map(keyOf), limit: rows.length })).items.forEach(r => have.add(rowKey(r)));
    const fresh = rows.filter(r => !have.has(rowKey(r)));
    if (!fresh.length) { say('info', t('dnd.allThere', { name })); return; }
    if (isCase) await api.addToCase(target.id, fresh.map(keyOf)); else await api.addToCollection(target.id, fresh.map(keyOf));
    useUiStore.getState().bumpLibrary(false);
    const undo = {
      label: t('dnd.undo'),
      run: () => {
        (isCase ? api.removeFromCase(target.id, fresh.map(keyOf)).then(() => undefined) : api.removeFromCollection(target.id, fresh.map(keyOf))).then(
          () => { say('info', t('dnd.undone', { what: what(fresh), name })); useUiStore.getState().bumpLibrary(!isCase && useUiStore.getState().view === `col:${target.id}`); },
          e => say('error', t('dnd.undoFail', { msg: (e as Error)?.message ?? String(e) })),
        );
      },
    };
    say('info', fresh.length === rows.length ? t('faces.addedTo', { what: what(rows), name }) : t('dnd.addedSome', { what: what(fresh), name, k: rows.length - fresh.length }), undo);
  } catch (e) { say('error', t('dnd.addFail', { name, msg: (e as Error)?.message ?? String(e) })); }
}
/** Tên cũ (chỉ dự án), giữ để nơi gọi và test hiện có không đổi. */
export const dropFacesIntoCollection = (api: Api, rows: readonly FaceRow[], target: { id: number; name: string }): Promise<void> => dropFacesInto(api, rows, { kind: 'col', ...target });
