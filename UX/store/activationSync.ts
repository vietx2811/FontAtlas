import type { Api } from '../api';
import { reconcileNotices } from '../components/activation/messages';
import { t } from '../i18n';
import { useAppPickerStore } from './appPicker';
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
    else if (e.type === 'appSet') { void appSetNotice(api, e.event); void refresh(); }
    else if (e.type === 'reconciled') {
      reconcileNotices(e.report).forEach(m => useNoticeStore.getState().push(m));
      void refresh();
    }
  }, 'activation.sync');
  await refresh();
  // Bên khác (FontBase, hệ thống…) có thể thêm/gỡ font lúc ta không nhìn: quay lại cửa sổ thì tính lại.
  const onFocus = () => { void refresh(); };
  window.addEventListener('focus', onFocus);
  return () => { window.removeEventListener('focus', onFocus); off(); };
}

/** B5: toast khi font của Case tự bật/tắt theo app. Không có gì đổi và không xung đột thì không báo (đỡ ồn). Xung đột/lỗi thì nói thật. */
export async function appSetNotice(api: Api, ev: { caseId: number; kind: 'on' | 'off' | string; changed: number; conflicts: number; failed: number }): Promise<void> {
  const key = `app:${ev.caseId}`;
  if (ev.kind === 'off') useNoticeStore.getState().dismissKey(key); // app đích đã đóng: toast "đã bật" của Case này hết còn đúng, dù lần tắt có đổi gì hay không
  if (ev.changed === 0 && ev.conflicts === 0 && ev.failed === 0) return;
  let name = `#${ev.caseId}`;
  try { name = (await api.getCase(ev.caseId)).name; } catch { /* Case vừa bị xoá: vẫn báo bằng số */ }
  const parts: string[] = [];
  if (ev.changed > 0) parts.push(t(ev.kind === 'on' ? 'app.on' : 'app.off', { n: ev.changed, name }));
  if (ev.conflicts > 0) parts.push(t('app.conflicts', { n: ev.conflicts }));
  if (ev.failed > 0) parts.push(t('app.failed', { n: ev.failed }));
  const detail = ev.changed > 0 ? [t(ev.kind === 'on' ? 'app.onWhy' : 'app.offWhy'), ...(ev.kind === 'on' ? [t('appset.restart')] : [])].join('. ') : undefined;
  const settings = { label: t('app.settings'), run: () => useAppPickerStore.getState().open(ev.caseId, name) };
  // Bật tự động: [Tắt lần này] (chỉ gỡ lý do caseapp của lượt này; font còn lý do khác vẫn bật) + [Cài đặt…]. Tắt tự động: không hoàn tác; [Bật lại] bật cả Case thủ công.
  const turnOffNow = { label: t('app.suppress'), run: () => { api.caseAppSuppress(ev.caseId).then(() => { useNoticeStore.getState().push({ kind: 'info', text: t('app.suppressed', { name }) }); void refreshActivation(api); }, e => useNoticeStore.getState().push({ kind: 'error', text: t('act.fail', { msg: (e as Error)?.message ?? String(e) }) })); } };
  const turnBackOn = { label: t('app.turnOn'), run: () => { api.activationActivateCase(ev.caseId).then(() => void refreshActivation(api), e => useNoticeStore.getState().push({ kind: 'error', text: t('act.fail', { msg: (e as Error)?.message ?? String(e) }) })); } };
  useNoticeStore.getState().push({
    kind: ev.conflicts > 0 || ev.failed > 0 ? 'error' : 'info', text: parts.join('. '), key, ...(detail ? { detail } : {}),
    ...(ev.kind === 'on' ? { action: turnOffNow, action2: settings } : ev.changed > 0 ? { action: turnBackOn } : {}),
  });
}
