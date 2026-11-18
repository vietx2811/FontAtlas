import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NOTICE_TTL_MS, NoticeStack } from '../NoticeStack';
import { createNoticeStore } from '../../store/notices';

describe('NoticeStack', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('thông báo info tự ẩn sau TTL kể từ lúc nó xuất hiện, dù có thông báo khác đến sau; lỗi ở lại', () => {
    const store = createNoticeStore();
    render(<NoticeStack store={store} />);
    act(() => { store.getState().push({ kind: 'info', text: 'một' }); store.getState().push({ kind: 'error', text: 'lỗi' }); });
    act(() => { vi.advanceTimersByTime(NOTICE_TTL_MS - 1000); });
    act(() => { store.getState().push({ kind: 'info', text: 'hai' }); }); // không được làm lại đồng hồ của "một"
    act(() => { vi.advanceTimersByTime(1100); });
    expect(screen.queryByText('một')).toBeNull();
    expect(screen.getByText('hai')).toBeTruthy();
    expect(screen.getByText('lỗi')).toBeTruthy();
    act(() => { vi.advanceTimersByTime(NOTICE_TTL_MS); });
    expect(screen.queryByText('hai')).toBeNull();
    expect(screen.getByText('lỗi')).toBeTruthy();
  });

  it('chi tiết phụ là dấu "!" có aria-label, không thêm chữ vào dòng chính', () => {
    const store = createNoticeStore();
    render(<NoticeStack store={store} />);
    act(() => { store.getState().push({ kind: 'info', text: 'Đã bật 3 font', detail: 'App gắn với Case vừa mở' }); });
    expect(screen.getByRole('status').textContent).toBe('Đã bật 3 font!×');
    expect(screen.getByTestId('notice-detail').getAttribute('aria-label')).toBe('App gắn với Case vừa mở');
  });

  it('thông báo cùng key thay cái cũ; dismissKey gỡ cả nhóm', () => {
    const store = createNoticeStore();
    store.getState().push({ kind: 'info', text: 'a', key: 'k' });
    store.getState().push({ kind: 'info', text: 'b', key: 'k' });
    expect(store.getState().notices.map(n => n.text)).toEqual(['b']);
    store.getState().dismissKey('k');
    expect(store.getState().notices).toEqual([]);
  });
});
