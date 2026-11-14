import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { appSetNotice } from '../../../store/activationSync';
import { useAppPickerStore } from '../../../store/appPicker';
import { useNoticeStore } from '../../../store/notices';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { AppPickerHost, BROWSERS, pickerRows } from '../AppPicker';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('B5 — Mở cùng app', () => {
  beforeEach(() => { useNoticeStore.setState({ notices: [] }); useAppPickerStore.setState({ target: null }); });
  afterEach(cleanup);

  it('pickerRows: bỏ trình duyệt, nhóm thiết kế lên đầu, app đã gắn nhưng không còn cài vẫn hiện', () => {
    const rows = pickerRows([
      { bundleId: 'com.apple.Safari', name: 'Safari', path: '/', running: false },
      { bundleId: 'com.apple.Pages', name: 'Pages', path: '/', running: false },
      { bundleId: 'com.figma.Desktop', name: 'Figma', path: '/', running: false },
      { bundleId: 'com.adobe.Illustrator', name: 'Adobe Illustrator', path: '/', running: true },
    ], [{ bundleId: 'com.old.App', name: 'Old App' }]);
    expect(rows.map(r => r.name)).toEqual(['Adobe Illustrator', 'Figma', 'Old App', 'Pages']);
    expect(rows.find(r => r.name === 'Old App')?.installed).toBe(false);
  });

  it('chọn app rồi Lưu ghi đúng danh sách; Lưu bị khoá khi chưa đổi gì; thông báo có Hoàn tác trả danh sách cũ', async () => {
    const api = createMockApi(); const caseId = (await api.createCase('Khách X')).id;
    render(<AppPickerHost api={api} />); act(() => useAppPickerStore.getState().open(caseId, 'Khách X')); await settle();
    expect(screen.queryByText('Safari')).toBeNull(); // trình duyệt không được đề xuất
    expect(screen.getByTestId('app-web-note')).toBeTruthy();
    const save = screen.getByTestId('app-save') as HTMLButtonElement; expect(save.disabled).toBe(true);
    const rows = screen.getAllByTestId('app-row'); expect(rows[0].dataset.bundle).toBe('com.adobe.Illustrator');
    fireEvent.click(rows[0].querySelector('input')!); expect(save.disabled).toBe(false);
    fireEvent.click(save); await settle();
    expect(await api.caseApps(caseId)).toEqual([{ bundleId: 'com.adobe.Illustrator', name: 'Adobe Illustrator 2026' }]);
    expect(screen.queryByTestId('app-list')).toBeNull(); // hộp thoại đã đóng
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.text).toMatch(/Khách X.*tự bật font/); expect(n.action?.label).toBe('Hoàn tác');
    act(() => n.action!.run()); await settle();
    expect(await api.caseApps(caseId)).toEqual([]);
  });

  it('toast appSet: nói số font, xung đột/lỗi thành thông báo lỗi, không có gì đổi thì im lặng', async () => {
    const api = createMockApi();
    await appSetNotice(api, { collectionId: 77, kind: 'on', changed: 0, conflicts: 0, failed: 0 });
    expect(useNoticeStore.getState().notices).toHaveLength(0);
    await appSetNotice(api, { collectionId: 77, kind: 'on', changed: 12, conflicts: 0, failed: 0 });
    expect(useNoticeStore.getState().notices.at(-1)).toMatchObject({ kind: 'info' }); expect(useNoticeStore.getState().notices.at(-1)!.text).toMatch(/Đã bật 12 font.*khởi động lại/);
    await appSetNotice(api, { collectionId: 77, kind: 'off', changed: 3, conflicts: 2, failed: 1 });
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.kind).toBe('error'); expect(n.text).toMatch(/Đã tắt 3 font/); expect(n.text).toMatch(/2 font trùng tên/); expect(n.text).toMatch(/1 font không bật được/);
  });

  it('toast bật: có nút Cài đặt… mở hộp thoại của đúng Case; toast tắt: nút Bật lại bật cả Case', async () => {
    const api = createMockApi(); const id = (await api.createCase('Khách Y')).id;
    await appSetNotice(api, { collectionId: id, kind: 'on', changed: 5, conflicts: 0, failed: 0 });
    const on = useNoticeStore.getState().notices.at(-1)!; expect(on.action?.label).toBe('Tắt lần này'); expect(on.action2?.label).toBe('Cài đặt…');
    const sup = vi.spyOn(api, 'caseAppSuppress'); act(() => on.action!.run()); await settle(); expect(sup).toHaveBeenCalledWith(id); expect(useNoticeStore.getState().notices.at(-1)!.text).toMatch(/lần này.*tự bật lại/);
    act(() => on.action2!.run()); expect(useAppPickerStore.getState().target).toEqual({ caseId: id, name: 'Khách Y' });
    const spy = vi.spyOn(api, 'activationActivateCase');
    await appSetNotice(api, { collectionId: id, kind: 'off', changed: 5, conflicts: 0, failed: 0 });
    const off = useNoticeStore.getState().notices.at(-1)!; expect(off.action?.label).toBe('Bật lại'); expect(off.action2).toBeUndefined();
    act(() => off.action!.run()); await settle(); expect(spy).toHaveBeenCalledWith(id);
  });

  it('QA native dùng đúng luật trình duyệt (PREFIX) của app: hai bản regex phải khớp; "Eagle for Safari" không phải trình duyệt', () => {
    const qa = readFileSync(resolve(process.cwd(), 'src-tauri/tests/qa/main_flows.js'), 'utf8');
    expect(qa).toContain(BROWSERS.source);
    expect(BROWSERS.test('com.apple.Safari')).toBe(true);
    expect(BROWSERS.test('com.example.eagle.safari-extension')).toBe(false);
    expect(BROWSERS.test('com.example.EagleForSafari')).toBe(false);
  });
});
