import type { Api, DerivedCopyDto, FaceKeyDto } from '../../api';
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
