import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { CmdError, type CloudDownloadReportDto } from '../../../api/types';
import { setLocale } from '../../../i18n';
import { createCloudDownloadStore, cloudDownloadNotice, downloadSourceToMac, useCloudDownloadStore } from '../../../store/cloudDownload';
import { createNoticeStore, useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { Sidebar } from '../Sidebar';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 5; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const report = (o: Partial<CloudDownloadReportDto> = {}): CloudDownloadReportDto => ({ total: 12, downloaded: 12, failed: 0, cancelled: false, rescanQueued: true, firstFailureCode: null, ...o });

/** Mock api có "Tải về máy" treo tới khi test cho xong. */
function hangingApi() {
  const api = createMockApi();
  let finish: (r: CloudDownloadReportDto) => void = () => {};
  let progress: (p: { done: number; total: number; failed: number }) => void = () => {};
  api.sourceDownloadCloud = vi.fn((_id: number, onProgress?: (p: { done: number; total: number; failed: number }) => void) => {
    progress = onProgress ?? (() => {});
    return new Promise<CloudDownloadReportDto>(res => { finish = res; });
  });
  api.sourceDownloadCancel = vi.fn(async () => true);
  return { api, finish: (r: CloudDownloadReportDto) => finish(r), progress: (p: { done: number; total: number; failed: number }) => progress(p) };
}

describe('Tải về máy — Sidebar', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); useCloudDownloadStore.setState({ runs: {} }); });
  afterEach(() => setLocale('vi'));

  it('only a source with files still in the cloud gets the button; clicking asks the backend and shows progress with Cancel', async () => {
    const { api, progress } = hangingApi();
    render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    const start = screen.getAllByTestId('cloud-dl-start');
    expect(start).toHaveLength(1); // nguồn thường và nguồn mất kết nối không có nút
    fireEvent.click(start[0]); await settle();
    expect(api.sourceDownloadCloud).toHaveBeenCalledWith(2, expect.any(Function));
    expect(screen.getByTestId('cloud-dl-running').textContent).toMatch(/Đang chuẩn bị tải/);
    act(() => progress({ done: 3, total: 12, failed: 0 }));
    expect(screen.getByTestId('cloud-dl-running').textContent).toMatch(/Đang tải 3\/12/);
    expect(screen.queryByTestId('cloud-dl-start')).toBeNull();
    // bấm nút lần nữa khi đang tải không mở lần thứ hai
    fireEvent.click(within(screen.getByTestId('cloud-dl-running')).getByTestId('cloud-dl-cancel')); await settle();
    expect(api.sourceDownloadCancel).toHaveBeenCalledWith(2);
    expect(screen.getByTestId('cloud-dl-cancel').textContent).toBe('Đang huỷ…');
    expect((screen.getByTestId('cloud-dl-cancel') as HTMLButtonElement).disabled).toBe(true);
    expect(api.sourceDownloadCloud).toHaveBeenCalledTimes(1);
  });

  it('when it finishes the line goes back to the button and a notice says what happened', async () => {
    const { api, finish } = hangingApi();
    render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    fireEvent.click(screen.getByTestId('cloud-dl-start')); await settle();
    await act(async () => { finish(report()); }); await settle();
    expect(screen.queryByTestId('cloud-dl-running')).toBeNull();
    expect(useNoticeStore.getState().notices.map(n => n.text)).toEqual(['Đã tải 12 font của “Dự án khách” về máy; đang quét lại để chúng hiện ra.']);
  });

  it('the source menu has Tải về máy for a cloud source (and Huỷ while running), none for a local source', async () => {
    const { api } = hangingApi();
    render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Fonts/ })); await settle();
    expect(screen.queryByRole('menuitem', { name: /Tải về máy/ })).toBeNull();
    fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' }); await settle();
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Dự án khách/ })); await settle();
    const item = screen.getByRole('menuitem', { name: /^Tải về máy/ });
    expect(item.getAttribute('aria-disabled')).not.toBe('true');
    fireEvent.click(item); await settle();
    expect(api.sourceDownloadCloud).toHaveBeenCalledWith(2, expect.any(Function));
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Dự án khách/ })); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: 'Huỷ tải về máy' })); await settle();
    expect(api.sourceDownloadCancel).toHaveBeenCalledWith(2);
  });

  it('Hiện trong Finder on a source asks for the SOURCE by id (not revealLocation with the folder path) and surfaces the error text', async () => {
    const api = createMockApi();
    const reveal = vi.spyOn(api, 'revealSource');
    const legacy = vi.spyOn(api, 'revealLocation');
    render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Dự án khách/ })); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: 'Hiện trong Finder' })); await settle();
    expect(reveal).toHaveBeenCalledWith(2);
    expect(legacy).not.toHaveBeenCalled();
    reveal.mockRejectedValueOnce(new CmdError('source_unavailable', 'Thư mục của nguồn hiện không truy cập được'));
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Dự án khách/ })); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: 'Hiện trong Finder' })); await settle();
    expect(document.querySelector('.fa-notice')?.textContent).toMatch(/không truy cập được/);
  });

  it('English UI: button, progress and menu are translated', async () => {
    setLocale('en');
    const { api, progress } = hangingApi();
    render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    expect(screen.getByTestId('cloud-dl-start').textContent).toBe('Download now');
    fireEvent.click(screen.getByTestId('cloud-dl-start')); await settle();
    act(() => progress({ done: 1, total: 12, failed: 0 }));
    expect(screen.getByTestId('cloud-dl-running').textContent).toMatch(/Downloading 1\/12/);
    expect(screen.getByTestId('cloud-dl-cancel').textContent).toBe('Cancel');
  });
});

describe('Tải về máy — store', () => {
  afterEach(() => setLocale('vi'));

  it('maps every report shape to the right notice (vi)', () => {
    expect(cloudDownloadNotice(report({ total: 0, downloaded: 0 }), 'A')).toEqual({ kind: 'info', text: '“A” không còn font nào chưa tải về.' });
    expect(cloudDownloadNotice(report({ total: 10, downloaded: 4, cancelled: true }), 'A')).toEqual({ kind: 'info', text: 'Đã huỷ: tải được 4/10 font của “A”.' });
    expect(cloudDownloadNotice(report({ total: 5, downloaded: 0, failed: 5, firstFailureCode: 'cloud_timeout' }), 'A').kind).toBe('error');
    expect(cloudDownloadNotice(report({ total: 5, downloaded: 0, failed: 5, firstFailureCode: 'cloud_timeout' }), 'A').text).toMatch(/Không tải được font nào của “A” \(hết thời gian chờ/);
    const partial = cloudDownloadNotice(report({ total: 5, downloaded: 3, failed: 2, firstFailureCode: 'cloud_stub' }), 'A');
    expect(partial.kind).toBe('error');
    expect(partial.text).toBe('Tải được 3 font của “A”, còn 2 font chưa tải được (iCloud chưa cho tải file — mở thư mục trong Finder rồi thử lại).');
    expect(cloudDownloadNotice(report({ total: 5, downloaded: 0, failed: 5, firstFailureCode: 'mới_chưa_biết' }), 'A').text).toMatch(/lỗi không rõ/);
  });

  it('English notices, with singular/plural', () => {
    setLocale('en');
    expect(cloudDownloadNotice(report({ total: 1, downloaded: 1 }), 'A').text).toBe('Downloaded 1 font from “A”; rescanning so it appears.');
    expect(cloudDownloadNotice(report({ total: 3, downloaded: 3 }), 'A').text).toBe('Downloaded 3 fonts from “A”; rescanning so they appear.');
    expect(cloudDownloadNotice(report({ total: 5, downloaded: 2, failed: 3, firstFailureCode: 'cloud_download_failed' }), 'A').text).toMatch(/^Downloaded 2 fonts from “A”; 3 couldn’t be downloaded \(macOS reported an error/);
  });

  it('a backend error becomes a notice, the run is cleared, and a second start while running is ignored', async () => {
    const api = createMockApi();
    const store = createCloudDownloadStore();
    const notices = createNoticeStore();
    api.sourceDownloadCloud = vi.fn(async () => { throw new CmdError('source_unavailable', 'Thư mục của nguồn hiện không truy cập được'); });
    const r = await downloadSourceToMac(api, { id: 3, name: 'Ổ ngoài' }, store, notices);
    expect(r).toBeNull();
    expect(store.getState().runs).toEqual({});
    expect(notices.getState().notices[0]).toMatchObject({ kind: 'error', text: expect.stringMatching(/Không tải về máy được “Ổ ngoài”: Thư mục của nguồn/) });
    // đang chạy → bỏ qua
    store.getState().set(2, { done: 0, total: 0, failed: 0, cancelling: false });
    const spy = vi.spyOn(api, 'sourceDownloadCloud');
    spy.mockClear();
    expect(await downloadSourceToMac(api, { id: 2, name: 'X' }, store, notices)).toBeNull();
    expect(spy).not.toHaveBeenCalled();
  });
});
