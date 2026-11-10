/** Chuỗi cho font nằm trên ổ cloud (OneDrive, Google Drive, Dropbox, Box, iCloud) — bản 0.2.1. */
export const CLOUD_I18N = {
  'cloud.loading': 'Trên cloud — đang tải về để xem trước…',
  'cloud.errTimeout': 'Hết 30 giây chờ tải font từ cloud nên chưa có preview. Kiểm tra mạng hoặc ứng dụng ổ cloud; kéo lại thẻ này để thử lại.',
  'cloud.errFailed': 'Không tải được font từ cloud nên chưa có preview. Kiểm tra mạng hoặc ứng dụng ổ cloud.',
  'cloud.errOff': 'Font chưa tải về máy nên chưa có preview. Bật “Tự tải font trên cloud để xem trước” trong Cài đặt → Thư viện.',
  'cloud.sourceHint': '{pct}% font của nguồn này chưa tải về máy — chọn Always keep on this device trong Finder để xem nhanh hơn.',
  'cloud.switch': 'Tự tải font trên cloud để xem trước',
  'cloud.switch.note': 'Bật: thẻ nào đang hiện mà font còn trên cloud thì được tải về để vẽ preview (tối đa 4 font cùng lúc, mỗi font chờ tối đa 30 giây). Tắt: không tải gì; thẻ đó chưa có preview cho tới khi bạn tải font về.',
  'cloud.note': 'Trên cloud — chưa tải về máy. Font sẽ tự tải khi bạn bật, tối đa 30 giây.',
  'act.msg.cloudTimeout': 'Hết 30 giây chờ tải font từ ổ cloud. Kiểm tra mạng hoặc ứng dụng ổ cloud rồi thử lại.',
  'act.msg.cloudFailed': 'Không tải được font từ ổ cloud: {message}',
  'act.msg.cloudStub': 'Font mới có bản giữ chỗ iCloud, chưa có file để tải. Mở thư mục trong Finder để iCloud tải về rồi thử lại.',
  'term.dataless.gloss': 'File chỉ còn bản giữ chỗ trên máy: nội dung nằm trên cloud (OneDrive, iCloud…) và được tải về khi có app mở nó.',
  'term.always_keep.gloss': 'Tuỳ chọn của Finder (chuột phải thư mục → Always keep on this device): giữ file luôn nằm trên máy, không bị giải phóng dung lượng.',
} as const;
