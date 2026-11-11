import type { Api } from '../api';
import { reconcileNotices } from '../components/activation/messages';
import { useNoticeStore } from './notices';
import { useUiStore } from './ui';

/** Nạp lại cả "đang bật" (do Font Atlas) lẫn "có sẵn trên máy" (do bên khác); lỗi chỉ ghi log. */
export async function refreshActivation(api: Api): Promise<void> {
  const ui = useUiStore.getState();
  await Promise.all([
    ui.refreshActive(api).catch(e => console.warn('activation_state failed', e)),
    ui.refreshAvailable(api).catch(e => console.warn('activation_availability failed', e)),
  ]);
}

/** Đăng ký kênh kích hoạt: cập nhật bản đồ "đang bật" và báo kết quả đối soát/tự gỡ cho người dùng. */
export async function startActivationSync(api: Api): Promise<() => void> {
  const refresh = () => refreshActivation(api);
  const off = await api.activationSubscribe(e => {
    if (e.type === 'changed') void refresh();
    else if (e.type === 'reconciled') {
      reconcileNotices(e.report).forEach(m => useNoticeStore.getState().push(m));
      void refresh();
    }
  });
  await refresh();
  // Bên khác (FontBase, hệ thống…) có thể thêm/gỡ font lúc ta không nhìn: quay lại cửa sổ thì tính lại.
  const onFocus = () => { void refresh(); };
  window.addEventListener('focus', onFocus);
  return () => { window.removeEventListener('focus', onFocus); off(); };
}
