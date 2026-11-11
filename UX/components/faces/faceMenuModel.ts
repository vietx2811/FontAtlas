// Nội dung menu ngữ cảnh cho một hoặc nhiều font. Thuần dữ liệu để kiểm thử: hành động thật nằm trong FaceActions.
import type { CollectionDto, TagDto } from '../../api';
import type { FaceRow } from '../../features/library-grid/types';
import type { View } from '../../store/ui';
import type { MenuEntry } from '../ui/Menu';
import { t, formatNumber } from '../../i18n';

export type FaceMenuActions = {
  openDetail(row: FaceRow): void;
  viewFamily(row: FaceRow): void;
  findSimilar(row: FaceRow): void;
  pairWith(row: FaceRow): void;
  setFavorite(rows: readonly FaceRow[], on: boolean): void;
  addToProject(rows: readonly FaceRow[], id: number, name: string): void;
  newProject(rows: readonly FaceRow[]): void;
  tag(rows: readonly FaceRow[], id: number, name: string): void;
  newTag(rows: readonly FaceRow[]): void;
  setActive(rows: readonly FaceRow[], on: boolean): void;
  reveal(row: FaceRow): void;
  copyNames(rows: readonly FaceRow[]): void;
  removeFromProject(rows: readonly FaceRow[], id: number): void;
};

export type FaceMenuCtx = {
  view: View;
  isFavorite(row: FaceRow): boolean;
  isActive(row: FaceRow): boolean;
  /** null = đang tải. */
  collections: readonly CollectionDto[] | null;
  tags: readonly TagDto[] | null;
  actions: FaceMenuActions;
};

export const faceName = (r: Pick<FaceRow, 'family' | 'subfamily'>): string => `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}`;

export function projectEntries(rows: readonly FaceRow[], ctx: Pick<FaceMenuCtx, 'collections' | 'actions' | 'view'>): MenuEntry[] {
  const { collections, actions } = ctx;
  // Đang xem chính dự án đó thì thêm vào là vô nghĩa.
  const here = ctx.view.startsWith('col:') ? Number(ctx.view.slice(4)) : null;
  const list: MenuEntry[] = collections === null
    ? [{ id: 'loading', label: t('faces.loading'), disabled: true }]
    : collections.map(c => ({ id: `col-${c.id}`, label: c.name, hint: String(c.itemCount), disabled: c.id === here, reason: t('faces.hereReason'), onSelect: () => actions.addToProject(rows, c.id, c.name) }));
  return [...list, ...(list.length ? [{ type: 'sep' as const, id: 'sep-new' }] : []), { id: 'new-project', label: t('faces.newProject'), onSelect: () => actions.newProject(rows) }];
}

export function buildFaceMenu(rows: readonly FaceRow[], ctx: FaceMenuCtx): MenuEntry[] {
  const { actions: a } = ctx;
  const n = rows.length;
  const one = n === 1 ? rows[0] : null;
  const out: MenuEntry[] = [];

  if (one) {
    out.push(
      { id: 'open', label: t('faces.open'), onSelect: () => a.openDetail(one) },
      { id: 'family', label: 'Xem family', onSelect: () => a.viewFamily(one) },
      { id: 'similar', label: t('faces.similar'), onSelect: () => a.findSimilar(one) },
      { id: 'pair', label: t('faces.pair'), onSelect: () => a.pairWith(one) },
      { type: 'sep', id: 'sep-nav' },
    );
  } else out.push({ type: 'label', id: 'count', text: t('faces.selectedN', { n: formatNumber(n) }) });

  const allFav = rows.every(ctx.isFavorite);
  out.push({ id: 'fav', label: allFav ? t('faces.unfav') : t('faces.fav'), onSelect: () => a.setFavorite(rows, !allFav) });
  out.push({ id: 'project', label: t('faces.addToProject'), items: projectEntries(rows, ctx) });
  const tagItems: MenuEntry[] = ctx.tags === null
    ? [{ id: 'loading', label: t('faces.loading'), disabled: true }]
    : ctx.tags.map(t => ({ id: `tag-${t.id}`, label: t.name, hint: String(t.faceCount), onSelect: () => a.tag(rows, t.id, t.name) }));
  out.push({ id: 'tag', label: t('faces.tag'), items: [...tagItems, ...(tagItems.length ? [{ type: 'sep' as const, id: 'sep-tag' }] : []), { id: 'new-tag', label: t('faces.newTag'), onSelect: () => a.newTag(rows) }] });

  const usable = rows.filter(r => !r.missing);
  const allActive = usable.length > 0 && usable.every(ctx.isActive);
  out.push({
    id: 'activate', label: allActive ? t('faces.deactivate') : t('faces.activate'), disabled: usable.length === 0, reason: t('faces.missingFile'),
    onSelect: () => a.setActive(usable, !allActive),
  });

  out.push({ type: 'sep', id: 'sep-file' });
  out.push({
    id: 'reveal', label: t('faces.reveal'), disabled: !one || !one.path || one.missing,
    reason: !one ? t('faces.revealNeedOne') : t('faces.missingFile'),
    onSelect: () => { if (one) a.reveal(one); },
  });
  out.push({ id: 'copy', label: t('faces.copyName'), onSelect: () => a.copyNames(rows) });
  if (ctx.view.startsWith('col:')) {
    const id = Number(ctx.view.slice(4));
    out.push({ id: 'unproject', label: t('faces.removeFromProject'), onSelect: () => a.removeFromProject(rows, id) });
  }
  return out;
}
