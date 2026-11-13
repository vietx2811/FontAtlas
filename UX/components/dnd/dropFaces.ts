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
export async function dropFacesInto(api: Api, rows: readonly FaceRow[], target: Pick<DropTarget, 'id' | 'name'> & { kind: 'col' | 'case' }): Promise<void> {
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

/** Thao tác cần thêm từ nơi thả (Library): bật/tắt font qua luồng kích hoạt chung (xung đột, hộp thoại). */
export type DropExtras = { setActive?(rows: readonly FaceRow[], on: boolean): Promise<void> };

/** Thả font lên thẻ: chỉ gắn font chưa có thẻ này; Hoàn tác gỡ đúng phần vừa gắn. */
async function dropFacesOnTag(api: Api, rows: readonly FaceRow[], target: Pick<DropTarget, 'id' | 'name'>): Promise<void> {
  const name = target.name;
  try {
    const have = new Set((await api.queryFaces({ tagIds: [target.id], faceKeys: rows.map(keyOf), limit: rows.length })).items.map(rowKey));
    const fresh = rows.filter(r => !have.has(rowKey(r)));
    if (!fresh.length) { say('info', t('dnd.tagAllThere', { name })); return; }
    await api.tagFaces(target.id, fresh.map(keyOf));
    useUiStore.getState().bumpLibrary(false);
    const undo = {
      label: t('dnd.undo'),
      run: () => {
        api.untagFaces(target.id, fresh.map(keyOf)).then(
          () => { say('info', t('dnd.tagUndone', { what: what(fresh), name })); useUiStore.getState().bumpLibrary(false); },
          e => say('error', t('dnd.undoFail', { msg: (e as Error)?.message ?? String(e) })),
        );
      },
    };
    say('info', fresh.length === rows.length ? t('faces.tagged', { name, what: what(rows) }) : t('dnd.tagSome', { what: what(fresh), name, k: rows.length - fresh.length }), undo);
  } catch (e) { say('error', t('faces.tagFail', { name, msg: (e as Error)?.message ?? String(e) })); }
}

/** Thả font lên "Đang bật": bật qua luồng chung (tự báo xung đột/lỗi); Hoàn tác tắt đúng những font vừa được bật. */
async function dropFacesOnActive(rows: readonly FaceRow[], extras: DropExtras): Promise<void> {
  const setActive = extras.setActive;
  if (!setActive) return;
  const before = new Set(useUiStore.getState().active.keys());
  const todo = rows.filter(r => !r.missing && !before.has(r.hashHex));
  if (!todo.length) { say('info', rows.every(r => r.missing) ? t('dnd.activeMissing') : t('dnd.activeAllThere')); return; }
  await setActive(todo, true);
  const now = useUiStore.getState().active;
  const done = todo.filter(r => now.has(r.hashHex));
  if (!done.length) return; // luồng chung đã nói lý do (xung đột, lỗi, đang chờ xác nhận)
  say('info', t('dnd.activeDone', { what: what(done) }), { label: t('dnd.undo'), run: () => void setActive(done, false) });
}

/** Cửa vào chung của lượt thả font từ lưới: chọn bộ xử lý theo loại đích. */
export function dropFacesOn(api: Api, rows: readonly FaceRow[], target: DropTarget, extras: DropExtras = {}): Promise<void> {
  if (target.kind === 'active') return dropFacesOnActive(rows, extras);
  if (target.kind === 'tag') return dropFacesOnTag(api, rows, target);
  return dropFacesInto(api, rows, { kind: target.kind, id: target.id, name: target.name });
}
