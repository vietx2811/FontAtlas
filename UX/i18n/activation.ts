/** Chuỗi cho Activation dễ thấy (công tắc Active, trạng thái, phạm vi, Top Bar, family). "Active" là thuật ngữ, giữ tiếng Anh; gloss ở term.activate.gloss. */
export const ACTIVATION_I18N = {
  'act.label': 'Active', 'act.switch.on': 'Tắt Active cho font này', 'act.switch.off': 'Bật Active cho font này',
  'act.status.session': 'Đang bật · phiên này', 'act.status.persistent': 'Luôn bật', 'act.status.off': 'Chưa bật',
  'act.scope.toPersistent': 'Giữ bật', 'act.scope.toSession': 'Chỉ phiên này',
  'act.scope.toPersistent.tip': 'Giữ font bật cả khi tắt Font Atlas', 'act.scope.toSession.tip': 'Tự tắt khi thoát Font Atlas',
  'act.scope.fail': 'Không đổi được phạm vi bật: {msg}',
  'act.card.on': 'Tắt Active', 'act.card.off': 'Bật Active', 'act.dot.session': 'Đang bật · phiên này', 'act.dot.persistent': 'Luôn bật',
  'act.family.on': 'Tắt Active cả family ({n} kiểu)', 'act.family.off': 'Bật Active cả family ({n} kiểu)',
  'act.family.label': 'Active cả family',
  'topbar.activate': 'Activate', 'topbar.deactivate': 'Deactivate',
  'topbar.activate.gloss': 'bật các font đã chọn', 'topbar.deactivate.gloss': 'tắt các font đã chọn',
} as const;
