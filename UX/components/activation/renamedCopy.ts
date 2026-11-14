import type { Api, ActivationReportDto, DerivedCopyDto, FaceKeyDto } from '../../api';
import { useNoticeStore } from '../../store/notices';
import { sayOutcomes } from './useActivationFlow';
import { CmdError } from '../../api';
import { t } from '../../i18n';
import { faceLabel } from '../../store/nameClash';

/** Lỗi của derive_renamed_copy → câu "chuyện gì + làm gì". */
export function deriveErrorText(e: unknown): string {
  const code = e instanceof CmdError ? e.code : '';
  const key = ({ not_variable: 'copy.err.notVariable', no_clash: 'copy.err.noClash', unsupported_container: 'copy.err.container', not_downloaded: 'copy.err.notDownloaded', disk_full: 'copy.err.disk', not_in_library: 'copy.err.notInLibrary' } as Record<string, string>)[code];
  return key ? t(key as 'copy.err.noClash') : t('copy.err.other', { msg: (e as Error)?.message ?? String(e) });
}

/** Tạo bản sao đổi tên và trả tên hiển thị của bản sao (vd "Oswald VF Regular"); tên lấy từ chính thư viện sau khi nạp, thiếu thì để trống. */
export async function makeRenamedCopy(api: Api, face: FaceKeyDto): Promise<{ copy: DerivedCopyDto; name: string }> {
  const copy = await api.deriveRenamedCopy(face);
  let name = '';
  try { const row = (await api.queryFaces({ faceKeys: [copy.face], limit: 1 })).items[0]; if (row) name = faceLabel(row); } catch { /* chưa nạp xong: bỏ tên */ }
  return { copy, name };
}

/** Câu thông báo sau khi tạo: tên mới, đã có từ trước hay vừa tạo, kèm ghi chú giấy phép hạn chế (không chặn). */
export function copyDoneText(copy: DerivedCopyDto, name: string): string {
  const base = name ? t(copy.created ? 'copy.done.named' : 'copy.exists.named', { name }) : t(copy.created ? 'copy.done' : 'copy.exists');
  return copy.licenseRestricted ? `${base} ${t('copy.restricted')}` : base;
}

/**
 * "Giữ cả hai" (G7b) qua backend: `activation_keep_both` biết VF nào đang bật nên tự tạo bản sao đổi tên và bật bản sao cùng static.
 * Toast nói tên bản sao ("X VF Kiểu") và ghi chú giấy phép hạn chế (không chặn); tạo bản sao lỗi thì nói lý do và rằng đã bật như cũ (trùng tên).
 */
export async function keepBoth(api: Api, faces: FaceKeyDto[]): Promise<ActivationReportDto> {
  const rep = await api.activationKeepBoth(faces, null);
  sayOutcomes(rep.results.map(r => r.outcome));
  const push = useNoticeStore.getState().push;
  const copied = rep.results.filter(r => r.warning === 'renamed_copy' || r.warning === 'renamed_copy_restricted');
  if (copied.length) {
    let rows: Awaited<ReturnType<Api['queryFaces']>>['items'] = [];
    try { rows = (await api.queryFaces({ faceKeys: copied.map(r => ({ hashHex: r.hashHex, faceIndex: r.requestedFaces[0] ?? 0 })), limit: copied.length })).items; } catch { /* không có tên: câu chung */ }
    const names = rows.map(r => [`${r.family} VF`, r.subfamily].filter(Boolean).join(' '));
    const list = names.slice(0, 3).map(n => `«${n}»`).join(', ') + (names.length > 3 ? ` +${names.length - 3}` : '');
    const restricted = copied.some(r => r.warning === 'renamed_copy_restricted');
    push({ kind: 'info', text: (names.length ? t('copy.both.named', { names: list }) : t('copy.both')) + (restricted ? ` ${t('copy.restricted')}` : '') });
  }
  const failed = rep.results.filter(r => r.warning?.startsWith('rename_failed:'));
  if (failed.length) push({ kind: 'error', text: `${deriveErrorText(new CmdError(failed[0].warning!.slice('rename_failed:'.length), ''))} ${t('copy.both.fallback')}` });
  return rep;
}
