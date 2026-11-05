import type { Api } from '../api';
import { reconcileNotices } from '../components/activation/messages';
import { useNoticeStore } from './notices';
import { useUiStore } from './ui';

/** Đăng ký kênh kích hoạt: cập nhật bản đồ "đang bật" và báo kết quả đối soát/tự gỡ cho người dùng. */
export async function startActivationSync(api: Api): Promise<() => void> {
  const refresh = () => useUiStore.getState().refreshActive(api).catch(e => console.warn('activation_state failed', e));
  const off = await api.activationSubscribe(e => {
    if (e.type === 'changed') void refresh();
    else if (e.type === 'reconciled') {
      reconcileNotices(e.report).forEach(m => useNoticeStore.getState().push(m));
      void refresh();
    }
  });
  await refresh();
  return off;
}
