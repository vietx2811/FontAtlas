/**
 * Ngôn ngữ định dạng số/ngày của trang Trùng lặp. Tạm đọc `<html lang>`; khi `feat/i18n-en` vào dev/mvp thì thay bằng
 * `localeTag()` / `formatNumber` / `formatDate` của `i18n/locale.ts` (cùng tên hàm, đổi import là xong).
 */
export const localeTag = (): string => (typeof document !== 'undefined' && document.documentElement.lang.toLowerCase().startsWith('en') ? 'en-US' : 'vi-VN');
