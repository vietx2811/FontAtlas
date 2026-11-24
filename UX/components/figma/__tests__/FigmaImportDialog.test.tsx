import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { setLocale } from '../../../i18n';
import { useFigmaImport } from '../../../store/figmaImport';
import { useNoticeStore } from '../../../store/notices';
import { FigmaImportHost } from '../FigmaImportHost';

const r0 = mockRows()[0];
const payload = (fonts: unknown[], file = 'Menu v3') => JSON.stringify({ v: 1, src: 'figma', file, fonts });

beforeEach(() => { setLocale('vi'); useFigmaImport.getState().close(); });
afterEach(() => { useFigmaImport.getState().close(); });

describe('Dán từ Figma', () => {
  it('dán → xem trước (exact chọn sẵn, none không vào Case) → tạo Case chỉ với font đã chọn', async () => {
    const api = createMockApi();
    render(<FigmaImportHost api={api} />);
    act(() => useFigmaImport.getState().openPaste());
    fireEvent.change(screen.getByLabelText('Nội dung sao chép từ plugin Figma'), { target: { value: payload([[r0.family, r0.subfamily, 0], ['Font Không Tồn Tại', 'Regular', 1]]) } });
    fireEvent.click(screen.getByRole('button', { name: 'Đọc' }));
    expect(await screen.findByText(/Tìm thấy 2 font: 1 có trong thư viện, 0 cần chọn bản, 1 chưa có\./)).toBeTruthy();
    const boxes = screen.getAllByRole('checkbox').filter(b => (b as HTMLInputElement).getAttribute('aria-label')?.startsWith('Đưa'));
    expect((boxes[0] as HTMLInputElement).checked).toBe(true);
    expect((boxes[1] as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByText('Không có trong thư viện')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Tạo Case (1 font)' }));
    await waitFor(async () => expect((await api.listCases(false)).some(c => c.name === 'Figma: Menu v3' && c.faceCount === 1)).toBe(true));
    expect(useFigmaImport.getState().phase).toBe('closed');
    expect(useNoticeStore.getState().notices.some(n => n.text.includes('Menu v3'))).toBe(true);
  });

  it('có thể bỏ tên file Figma khỏi tên Case', async () => {
    const api = createMockApi();
    render(<FigmaImportHost api={api} />);
    act(() => useFigmaImport.getState().show({ file: 'Riêng tư', items: [{ family: r0.family, style: r0.subfamily ?? '', figmaMissing: false }] }));
    fireEvent.click(await screen.findByLabelText('Không dùng tên file Figma'));
    fireEvent.click(await screen.findByRole('button', { name: 'Tạo Case (1 font)' }));
    await waitFor(async () => expect((await api.listCases(false)).some(c => c.name === 'Từ Figma')).toBe(true));
  });

  it('nội dung hỏng hiện lỗi theo mã, tiếng Anh khi đổi ngôn ngữ, và không tạo gì', async () => {
    const api = createMockApi();
    render(<FigmaImportHost api={api} />);
    act(() => useFigmaImport.getState().openPaste());
    fireEvent.change(screen.getByLabelText('Nội dung sao chép từ plugin Figma'), { target: { value: 'không phải json' } });
    fireEvent.click(screen.getByRole('button', { name: 'Đọc' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Nội dung dán không đúng định dạng của plugin Font Atlas cho Figma.');
    expect((await api.listCases(false)).length).toBe(0);
    act(() => setLocale('en'));
    fireEvent.click(screen.getByRole('button', { name: 'Read' }));
    expect((await screen.findByRole('alert')).textContent).toBe('That doesn’t look like the Font Atlas Figma plugin’s format.');
  });

  it('link mở lạnh: lấy hàng đợi lúc khởi động và mở hộp xem trước; link hỏng báo lỗi', async () => {
    const base = createMockApi();
    const pending = [
      { import: null, error: 'import_version' },
      { import: { file: 'Cold', items: [{ family: r0.family, style: r0.subfamily ?? '', figmaMissing: false }] }, error: null },
    ];
    let first = true;
    const api = { ...base, takePendingImports: async () => { if (!first) return []; first = false; return pending; } };
    render(<FigmaImportHost api={api} />);
    expect(await screen.findByText(/Tìm thấy 1 font/)).toBeTruthy();
    expect(useNoticeStore.getState().notices.some(n => n.kind === 'error' && n.text.includes('phiên bản plugin mới hơn'))).toBe(true);
  });
});
