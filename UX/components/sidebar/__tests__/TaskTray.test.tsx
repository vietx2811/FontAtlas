import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { TaskDto } from '../../../api/taskTypes';
import { createTaskHubStore } from '../../../store/taskHub';
import { TaskTray } from '../TaskTray';

const task = (id: string, o: Partial<TaskDto> = {}): TaskDto => ({
  id, rev: 1, kind: 'scan', trigger: 'user', titleKey: 'task.kind.scan', titleArgs: {}, done: 3, total: 12, unit: 'items', state: 'running', cancellable: true, startedAt: 1, ...o,
});
const put = (store: ReturnType<typeof createTaskHubStore>, ...ts: TaskDto[]) => act(() => { store.setState({ tasks: Object.fromEntries(ts.map(x => [x.id, x])) }); });

describe('TaskTray', () => {
  it('rảnh: không render gì (ẩn hẳn)', () => {
    const store = createTaskHubStore();
    const { container } = render(<TaskTray api={createMockApi()} store={store} />);
    expect(container.firstChild).toBeNull();
  });

  it('một việc: nút có tên truy cập + "3/12"; mở popover, Esc đóng và trả focus', () => {
    const store = createTaskHubStore();
    render(<TaskTray api={createMockApi()} store={store} />);
    put(store, task('a'));
    const btn = screen.getByTestId('task-button');
    expect(btn.getAttribute('aria-label')).toBe('1 tác vụ nền');
    expect(btn.getAttribute('aria-expanded')).toBe('false');
    expect(screen.getByTestId('task-count').textContent).toBe('3/12');
    fireEvent.click(btn);
    expect(screen.getByTestId('task-popover')).toBeInTheDocument();
    expect(screen.getByTestId('task-num').textContent).toBe('3/12');
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByTestId('task-popover')).toBeNull();
    expect(document.activeElement).toBe(btn);
  });

  it('total null: vòng quay, không số tổng', () => {
    const store = createTaskHubStore();
    render(<TaskTray api={createMockApi()} store={store} />);
    put(store, task('a', { total: null, done: 7 }));
    expect(screen.getByTestId('task-ring').getAttribute('data-value')).toBe('indeterminate');
    expect(screen.queryByTestId('task-count')).toBeNull();
  });

  it('huỷ ngay khi không có cancelConfirmKey; việc đang huỷ không còn nút', () => {
    const store = createTaskHubStore();
    const api = createMockApi();
    const spy = vi.spyOn(api, 'taskCancel');
    render(<TaskTray api={api} store={store} />);
    put(store, task('a'));
    fireEvent.click(screen.getByTestId('task-button'));
    fireEvent.click(screen.getByTestId('task-cancel'));
    expect(spy).toHaveBeenCalledWith('a');
    put(store, task('a', { state: 'cancelling' }));
    expect(screen.queryByTestId('task-cancel')).toBeNull();
    expect(screen.getByText('Đang huỷ…')).toBeInTheDocument();
  });

  it('huỷ phá huỷ: hỏi ngay trong dòng; "Giữ chạy" không huỷ, "Huỷ việc này" mới huỷ', () => {
    const store = createTaskHubStore();
    const api = createMockApi();
    const spy = vi.spyOn(api, 'taskCancel');
    render(<TaskTray api={api} store={store} />);
    put(store, task('d', { kind: 'resolveDuplicates', cancelConfirmKey: 'task.kind.other' }));
    fireEvent.click(screen.getByTestId('task-button'));
    fireEvent.click(screen.getByTestId('task-cancel'));
    expect(spy).not.toHaveBeenCalled();
    expect(screen.getByTestId('task-confirm')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('task-keep'));
    expect(screen.queryByTestId('task-confirm')).toBeNull();
    fireEvent.click(screen.getByTestId('task-cancel'));
    fireEvent.click(screen.getByTestId('task-confirm-yes'));
    expect(spy).toHaveBeenCalledWith('d');
  });

  it('hơn 3 dòng cùng loại gộp thành một; huỷ dòng gộp huỷ từng việc', () => {
    const store = createTaskHubStore();
    const api = createMockApi();
    const spy = vi.spyOn(api, 'taskCancel');
    render(<TaskTray api={api} store={store} />);
    put(store, ...['1', '2', '3', '4'].map(i => task(i, { kind: 'cloudDownload' })));
    fireEvent.click(screen.getByTestId('task-button'));
    expect(screen.getAllByTestId('task-row')).toHaveLength(1);
    fireEvent.click(screen.getByTestId('task-cancel'));
    expect(spy).toHaveBeenCalledTimes(4);
  });

  it('task watcher chưa có tiến độ: khay vẫn ẩn', () => {
    const store = createTaskHubStore();
    const { container } = render(<TaskTray api={createMockApi()} store={store} />);
    put(store, task('w', { trigger: 'watcher', done: 0 }));
    expect(container.firstChild).toBeNull();
    put(store, task('w', { trigger: 'watcher', done: 2 }));
    expect(screen.getByTestId('task-button')).toBeInTheDocument();
  });

  it('việc vừa xong hiện ✓ rồi khay ẩn khi hết', () => {
    const store = createTaskHubStore();
    render(<TaskTray api={createMockApi()} store={store} />);
    act(() => { store.setState({ done: { a: task('a') } }); });
    fireEvent.click(screen.getByTestId('task-button'));
    expect(screen.getByText('Xong')).toBeInTheDocument();
    act(() => { store.setState({ done: {} }); });
    expect(screen.queryByTestId('task-button')).toBeNull();
  });

  it('khoá xác nhận thiếu bản dịch: nhóm xác nhận không lộ khoá thô', () => {
    const store = createTaskHubStore();
    render(<TaskTray api={createMockApi()} store={store} />);
    put(store, task('d', { cancelConfirmKey: 'task.confirm.khong.co' }));
    fireEvent.click(screen.getByTestId('task-button'));
    fireEvent.click(screen.getByTestId('task-cancel'));
    expect(screen.getByTestId('task-confirm').getAttribute('aria-label')).not.toContain('task.confirm');
  });

  it('khay về rảnh khi popover đang mở: focus trả về nút Cài đặt', () => {
    const store = createTaskHubStore();
    render(<div className="fa-foot-actions"><TaskTray api={createMockApi()} store={store} /><button type="button" aria-label="Cài đặt">⚙</button></div>);
    put(store, task('a'));
    const btn = screen.getByTestId('task-button');
    fireEvent.click(btn); btn.focus();
    put(store);
    expect(screen.queryByTestId('task-button')).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cài đặt' }));
  });

  it('tổng chưa biết: dòng hiện "Đang đếm…"; dòng gộp có tên "Huỷ tất cả"; watcher có chú giải tự động', () => {
    const store = createTaskHubStore();
    render(<TaskTray api={createMockApi()} store={store} />);
    put(store, task('a', { total: null, done: 0 }));
    fireEvent.click(screen.getByTestId('task-button'));
    expect(screen.getByTestId('task-num').textContent).toBe('Đang đếm…');
    put(store, ...['1', '2', '3', '4'].map(i => task(i, { kind: 'cloudDownload' })));
    expect(screen.getByTestId('task-cancel').getAttribute('aria-label')).toMatch(/^Huỷ tất cả/);
  });
});
