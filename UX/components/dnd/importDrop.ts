import type { Api } from '../../api';
import { CmdError } from '../../api';
import type { ImportProgressDto } from '../../api';
import { formatNumber, t, tn } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';

const say = (kind: 'info' | 'error', text: string, action?: { label: string; run(): void }): number => useNoticeStore.getState().push({ kind, text, action });
const fonts = (n: number): string => tn('faces.nFonts', { n: formatNumber(n) });

/** Câu mô tả tiến độ theo pha (một dòng, đổi theo từng pha/bước). */
export function importProgressText(p: ImportProgressDto, name: string): string {
  if (p.phase === 'adding_sources') return t('dnd.imp.sources', { name });
  if (p.phase === 'scanning') return t('dnd.imp.scan', { name, done: formatNumber(p.done), total: formatNumber(p.total) });
  return t('dnd.imp.adding', { name, n: formatNumber(p.added) });
}

/** Lý do bỏ qua phổ biến nhất, kèm số lượng, để thông báo nói thật vì sao một số file không vào. */
function skipSummary(skipped: ReadonlyArray<{ path: string; reason: string }>): string {
  const by = new Map<string, number>();
  for (const s of skipped) by.set(s.reason, (by.get(s.reason) ?? 0) + 1);
  return [...by.entries()].map(([r, n]) => t('dnd.imp.skip', { n: formatNumber(n), why: t(`dnd.imp.why.${r}` as 'dnd.imp.why.not_font') })).join('; ');
}

/**
 * Thả file/thư mục từ Finder vào một Dự án/Case (G8). Thông báo tiến độ có nút Huỷ; kết quả nói số font mới, số đã có, và vì sao
 * file nào bị bỏ qua; Hoàn tác gỡ đúng các face MỚI vừa thêm (không đụng font đã có sẵn). Không chặn theo giấy phép.
 * `onDone` để nơi gọi làm mới danh sách nguồn (có thể vừa thêm một Source hoặc nguồn "Font lẻ").
 */
export async function importPathsIntoTarget(api: Api, paths: string[], target: { kind: 'col' | 'case'; id: number; name: string }, onDone?: () => void): Promise<void> {
  const isCase = target.kind === 'case';
  const name = isCase ? `Case ${target.name}` : target.name;
  let progressId = say('info', importProgressText({ phase: 'adding_sources', done: 0, total: paths.length, added: 0, skipped: 0 }, name), { label: t('dnd.imp.cancel'), run: () => void api.importPathsCancel().catch(() => undefined) });
  const update = (p: ImportProgressDto) => {
    const id = useNoticeStore.getState().push({ kind: 'info', text: importProgressText(p, name), action: { label: t('dnd.imp.cancel'), run: () => void api.importPathsCancel().catch(() => undefined) } });
    if (id !== progressId) { useNoticeStore.getState().dismiss(progressId); progressId = id; }
  };
  try {
    const res = await api.importPathsInto(paths, { kind: isCase ? 'case' : 'collection', id: target.id }, update);
    useNoticeStore.getState().dismiss(progressId);
    useUiStore.getState().bumpLibrary(false); // font lẻ đổi thư viện mà không có sự kiện quét
    onDone?.();
    if (res.cancelled) { say('info', t('dnd.imp.cancelled', { name })); return; }
    const n = res.added.length;
    const skip = res.skipped.length ? ` ${t('dnd.imp.skipped', { list: skipSummary(res.skipped) })}` : '';
    if (!n) {
      if (res.alreadyThere > 0) say('info', t('dnd.imp.allThere', { name }) + skip);
      else say(res.skipped.length ? 'error' : 'info', t('dnd.imp.none', { name }) + skip);
      return;
    }
    const undo = {
      label: t('dnd.undo'),
      run: () => {
        (isCase ? api.removeFromCase(target.id, res.added) : api.removeFromCollection(target.id, res.added)).then(
          () => { say('info', t('dnd.imp.undone', { what: fonts(n), name })); useUiStore.getState().bumpLibrary(!isCase && useUiStore.getState().view === `col:${target.id}`); },
          e => say('error', t('dnd.undoFail', { msg: (e as Error)?.message ?? String(e) })),
        );
      },
    };
    const there = res.alreadyThere > 0 ? ` ${t('dnd.imp.there', { k: formatNumber(res.alreadyThere) })}` : '';
    say('info', t('dnd.imp.added', { what: fonts(n), name }) + there + skip, undo);
  } catch (e) {
    useNoticeStore.getState().dismiss(progressId);
    const code = e instanceof CmdError ? e.code : '';
    say('error', code === 'busy' ? t('dnd.imp.busy') : code === 'not_found' ? t('dnd.imp.gone', { name }) : t('dnd.imp.fail', { name, msg: (e as Error)?.message ?? String(e) }));
  }
}
