import { useEffect } from 'react';
import type { Api } from '../../api';
import { useFigmaImport } from '../../store/figmaImport';
import { useNoticeStore } from '../../store/notices';
import { t } from '../../i18n';
import { FigmaImportDialog, importErrorText } from './FigmaImportDialog';

/**
 * Nhận link fontatlas:// đã xếp hàng ở backend: lấy ngay lúc khởi động (mở lạnh) và mỗi khi có sự kiện (app đang chạy).
 * Chỉ MỞ hộp thoại xem trước; không làm gì khác. Link hỏng hiện thông báo lỗi theo mã.
 */
export function FigmaImportHost({ api }: { api: Api }): React.JSX.Element {
  useEffect(() => {
    let dead = false; let off: (() => void) | undefined;
    const take = async () => {
      const items = await api.takePendingImports().catch(() => []);
      if (dead) return;
      for (const it of items) if (it.error) useNoticeStore.getState().push({ kind: 'error', text: t('figma.linkError', { msg: importErrorText({ code: it.error }) }) });
      const last = [...items].reverse().find(i => i.import);
      if (last?.import) useFigmaImport.getState().show(last.import);
    };
    // Đăng ký nghe TRƯỚC rồi mới lấy hàng đợi: link tới giữa hai bước vẫn không mất.
    api.figmaImportSubscribe(() => { void take(); }).then(f => { if (dead) f(); else { off = f; void take(); } }).catch(() => void take());
    return () => { dead = true; off?.(); };
  }, [api]);
  return <FigmaImportDialog api={api} />;
}
