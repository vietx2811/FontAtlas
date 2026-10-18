import '@testing-library/jest-dom/vitest';

// jsdom thiếu các API trình duyệt mà lưới dùng.
if (!('ResizeObserver' in globalThis)) {
  (globalThis as any).ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
}
globalThis.requestAnimationFrame ??= ((cb: FrameRequestCallback) => setTimeout(() => cb(performance.now()), 0) as unknown as number);
globalThis.cancelAnimationFrame ??= ((h: number) => clearTimeout(h));
URL.createObjectURL ??= () => 'blob:mock';
URL.revokeObjectURL ??= () => {};
