// Yêu cầu mở Cài đặt thẳng vào một nhóm (vd. thẻ "Bật font tự động" ở bảng phím tắt). Settings đọc một lần khi dựng.
let want: string | null = null;
export const requestSettingsGroup = (id: string): void => { want = id; };
export const takeSettingsGroup = (): string | null => { const w = want; want = null; return w; };
