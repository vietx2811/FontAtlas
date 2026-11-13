import type { Api, FaceKeyDto, FileOutcomeDto } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { confirmedOf, type ConflictItem } from './conflictModel';
import { describeOutcome } from './messages';
import { restoreReplaced } from './restoreReplaced';

const say = (outcomes: FileOutcomeDto[]): void => outcomes.forEach(o => { const m = describeOutcome(o); if (m) useNoticeStore.getState().push(m); });
const APP_OF: Record<string, string> = { right_font: 'RightFont', font_base: 'FontBase', typeface: 'Typeface', adobe_fonts: 'Adobe Fonts', suitcase: 'Suitcase / Connect' };

/**
 * "Tắt bản cũ và bật bản này" cho các xung đột người dùng đã thấy (hộp thoại hoặc lựa chọn đã nhớ ở G7), kèm thông báo có nút khôi phục.
 * `retry`: xung đột đổi giữa lúc hỏi và lúc thay → hỏi lại với tập mới, không thay gì. `doneText`: thay lời "đã thay" mặc định.
 */
export async function runReplace(api: Api, faces: FaceKeyDto[], items: readonly ConflictItem[], retry: (faces: FaceKeyDto[]) => Promise<void>, doneText?: string): Promise<void> {
  const rep = await api.activationReplace(faces, confirmedOf(items));
  say(rep.results.map(r => r.outcome));
  const stale = rep.results.filter(r => r.outcome.kind === 'failed' && r.outcome.code === 'replace_conflicts_changed');
  if (stale.length) await retry(stale.map(r => ({ hashHex: r.hashHex, faceIndex: r.requestedFaces[0] ?? 0 })));
  const replaced = rep.results.flatMap(r => r.replaced ?? []);
  if (!replaced.length) return;
  const apps = [...new Set(replaced.map(x => APP_OF[x.owner] ?? t('act.replace.app.other')))].join(' / ');
  // thay xong (hoặc "không hiệu lực": sayOutcomes đã nói thật) — luôn có đường khôi phục
  const notEffective = rep.results.some(r => r.warning === 'replace_not_effective');
  useNoticeStore.getState().push({
    kind: 'info', text: notEffective ? t('act.msg.replaceNotEffective') : (doneText ?? t('act.replace.done', { app: apps })),
    action: { label: t('act.replace.undo'), run: () => void restoreReplaced(api, replaced.map(x => x.id)) },
  });
}
