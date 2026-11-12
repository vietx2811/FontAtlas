import '@testing-library/jest-dom/vitest';
import { cleanup, configure } from '@testing-library/react';
import { afterEach, beforeEach } from 'vitest';
import { resetChosenForTests, setLocale } from '../i18n/locale';

// Máy bận (CI, nhiều phiên song song) làm waitFor/findBy mặc định 1 s hụt: chờ có hạn dài, test đạt vẫn trả về ngay khi điều kiện đúng.
configure({ asyncUtilTimeout: 8000 });

// Vitest không bật globals → Testing Library không tự dọn DOM giữa các test.
afterEach(() => cleanup());
// Ngôn ngữ giao diện là trạng thái module: mỗi test bắt đầu ở tiếng Việt (mặc định của bộ test cũ) và không mang dấu "đã chọn" sang test khác.
beforeEach(() => { resetChosenForTests(); setLocale('vi'); });

// jsdom thiếu các API trình duyệt mà lưới dùng.
if (!('ResizeObserver' in globalThis)) {
  (globalThis as any).ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
}
// jsdom có rAF ~16 ms theo đồng hồ thật → thay bằng setTimeout(0) cho test tất định (và chạy được với đồng hồ giả).
globalThis.requestAnimationFrame = ((cb: FrameRequestCallback) => setTimeout(() => cb(performance.now()), 0) as unknown as number);
globalThis.cancelAnimationFrame = ((h: number) => clearTimeout(h));
URL.createObjectURL ??= () => 'blob:mock';
URL.revokeObjectURL ??= () => {};
