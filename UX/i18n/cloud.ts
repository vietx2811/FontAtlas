/** Chuỗi cho font nằm trên ổ cloud (OneDrive, Google Drive, Dropbox, Box, iCloud) — bản 0.2.1. */
export const CLOUD_I18N = {
  'cloud.note': 'Trên cloud — chưa tải về máy. Font sẽ tự tải khi bạn bật, tối đa 30 giây.',
  'act.msg.cloudTimeout': 'Hết 30 giây chờ tải font từ ổ cloud. Kiểm tra mạng hoặc ứng dụng ổ cloud rồi thử lại.',
  'act.msg.cloudFailed': 'Không tải được font từ ổ cloud: {message}',
  'act.msg.cloudStub': 'Font mới có bản giữ chỗ iCloud, chưa có file để tải. Mở thư mục trong Finder để iCloud tải về rồi thử lại.',
} as const;
