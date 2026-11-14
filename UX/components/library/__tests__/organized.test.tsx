import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { OrganizedPanel } from '../OrganizedPanel';
import { groupSummary, issueSummary } from '../OrganizedImportDialog';
import { relocateCounts, undoRelocate } from '../RelocateDialog';

const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const picker = (p: string | null) => async () => p;

describe('B7 — Organized Library (Cài đặt ▸ Thư viện)', () => {
  beforeEach(() => useNoticeStore.setState({ notices: [] }));
  afterEach(cleanup);

  it('chưa tạo: chọn thư mục → kiểm; ổ cloud chỉ CẢNH BÁO (vẫn tạo được); tạo xong hiện trạng thái', async () => {
    const api = createMockApi();
    render(<OrganizedPanel api={api} pick={picker('/Users/example/Library/CloudStorage/iCloud/Fonts')} />); await settle();
    fireEvent.click(screen.getByText('Chọn thư mục đặt thư viện…')); await settle();
    expect(screen.getByTestId('org-cloud-warn')).toBeTruthy();
    const create = screen.getByTestId('org-create') as HTMLButtonElement; expect(create.disabled).toBe(false);
    fireEvent.click(create); await settle();
    expect(screen.getByTestId('org-status').textContent).toMatch(/iCloud\/Fonts/);
    expect((await api.organizedStatus()).configured).toBe(true);
  });

  it('thư mục không ghi được hoặc nằm trong nguồn có sẵn: khoá nút Tạo và nói lý do', async () => {
    const api = createMockApi();
    render(<OrganizedPanel api={api} pick={picker('/Volumes/readonly/Fonts')} />); await settle();
    fireEvent.click(screen.getByText('Chọn thư mục đặt thư viện…')); await settle();
    expect((screen.getByTestId('org-create') as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole('alert').textContent).toMatch(/Không ghi được/);
  });

  it('chép: xem trước nói CHÉP không di chuyển + dung lượng, chạy có tiến độ, kết quả nêu số file và lỗi từng loại', async () => {
    const api = createMockApi(); await api.organizedCreate('/Users/example/Fonts');
    render(<OrganizedPanel api={api} />); await settle();
    fireEvent.click(screen.getByTestId('org-open-import')); await settle();
    expect(screen.getByTestId('org-copy-note').textContent).toMatch(/CHÉP, không di chuyển.*bản gốc nguyên vẹn.*tên file giữ nguyên/);
    expect(screen.getByTestId('org-plan').textContent).toMatch(/42/);
    fireEvent.click(screen.getByTestId('org-start')); await settle();
    expect(screen.getByTestId('org-done').textContent).toMatch(/Đã chép 42 file/);
    expect(screen.getByTestId('org-issues').textContent).toMatch(/1 chưa tải về máy/);
    expect((await api.organizedStatus()).files).toBe(42);
  });

  it('sắp xếp lại: xem trước từ → đến, chuyển rồi thông báo có Hoàn tác', async () => {
    const api = createMockApi(); await api.organizedCreate('/Users/example/Fonts');
    render(<OrganizedPanel api={api} />); await settle();
    fireEvent.click(screen.getByTestId('org-open-resort')); await settle();
    expect(screen.getByTestId('org-moves').textContent).toMatch(/Sans\/Foo Sans\/Foo\.ttf.*Slab\/Foo Sans\/Foo\.ttf/);
    fireEvent.click(screen.getByTestId('org-resort-apply')); await settle();
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.text).toMatch(/Đã sắp xếp lại 2 font/); expect(n.action?.label).toBe('Hoàn tác');
    act(() => n.action!.run()); await settle();
    expect(useNoticeStore.getState().notices.at(-1)!.text).toMatch(/Đã hoàn tác: 2 font về chỗ cũ/);
  });

  it('khôi phục từ gói: kiểm gói, xếp lịch, nhắc khởi động lại; gói hỏng báo rõ', async () => {
    const api = createMockApi();
    const { unmount } = render(<OrganizedPanel api={api} pick={picker('/Volumes/Backup/pack-bad')} />); await settle();
    fireEvent.click(screen.getByTestId('org-open-restore')); fireEvent.click(screen.getByTestId('org-pack-choose')); await settle();
    expect(screen.getByRole('alert').textContent).toMatch(/hỏng hoặc không phải gói/);
    unmount(); cleanup();
    render(<OrganizedPanel api={api} pick={picker('/Volumes/Backup/pack-good')} />); await settle();
    fireEvent.click(screen.getByTestId('org-open-restore')); fireEvent.click(screen.getByTestId('org-pack-choose')); await settle();
    expect(screen.getByTestId('org-pack-info').textContent).toMatch(/Font Atlas 0\.5\.0.*4\.812/);
    expect(screen.getByTestId('org-pack-remap')).toBeTruthy();
    fireEvent.click(screen.getByTestId('org-pack-stage')); await settle();
    expect(screen.getByTestId('org-pack-staged').textContent).toMatch(/thoát rồi mở lại/);
    expect(await api.packRestorePending()).not.toBeNull();
  });

  it('issueSummary gộp theo mã', () => {
    expect(issueSummary([{ code: 'cloud_not_downloaded' }, { code: 'cloud_not_downloaded' }, { code: 'verify_failed' }])).toBe('2 chưa tải về máy; 1 chép xong nhưng kiểm nội dung không khớp');
  });

  it('chuyển font đang bật: đếm kết quả theo file; không có font bật thì nói và khoá nút', async () => {
    expect(relocateCounts([{ outcome: { kind: 'activated' } }, { outcome: { kind: 'already_active' } }, { outcome: { kind: 'failed' } }, { outcome: { kind: 'missing_file' } }, { outcome: { kind: 'already_inactive' } }])).toEqual({ moved: 1, same: 1, kept: 2 });
    const api = createMockApi(); await api.organizedCreate('/Users/example/Fonts');
    render(<OrganizedPanel api={api} />); await settle();
    fireEvent.click(screen.getByTestId('org-open-relocate')); await settle();
    expect(screen.getByTestId('org-reloc-body').textContent).toMatch(/không có font nào đang bật/);
    expect((screen.getByTestId('org-reloc-run') as HTMLButtonElement).disabled).toBe(true);
  });

  it('Hoàn tác chuyển font: gọi activation_relocate_to với đường cũ đã ghi; báo số font không chuyển ngược được', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'activationRelocateTo').mockResolvedValue({ results: [{ hashHex: 'aa', requestedFaces: [0], outcome: { kind: 'activated' } }, { hashHex: 'bb', requestedFaces: [0], outcome: { kind: 'missing_file' } }] as never, changed: 1, needsConfirmation: 0, failed: 1 });
    await undoRelocate(api, [{ face: { hashHex: 'aa', faceIndex: 0 }, path: '/Users/example/Fonts/a.ttf' }, { face: { hashHex: 'bb', faceIndex: 0 }, path: '/Users/example/Fonts/b.ttf' }]);
    expect(spy).toHaveBeenCalledWith([{ face: { hashHex: 'aa', faceIndex: 0 }, path: '/Users/example/Fonts/a.ttf' }, { face: { hashHex: 'bb', faceIndex: 0 }, path: '/Users/example/Fonts/b.ttf' }]);
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.kind).toBe('error'); expect(n.text).toMatch(/1 font về đường cũ/); expect(n.text).toMatch(/1 font không chuyển ngược được/);
  });

  it('Tải về rồi chép: hiện khi có file chưa tải; bật thì tính lại; hết chỗ tải thì khoá; tiến độ tải; kết quả gộp theo mã với tổng thật', async () => {
    const api = createMockApi(); await api.organizedCreate('/Users/example/Fonts');
    render(<OrganizedPanel api={api} />); await settle();
    fireEvent.click(screen.getByTestId('org-open-import')); await settle();
    expect(screen.getByTestId('org-plan').textContent).toMatch(/Chưa tải về máy.*2 · /);
    const opt = screen.getByTestId('org-download-opt'); expect(opt.textContent).toMatch(/Tải về rồi chép \(2 file/); expect(opt.textContent).toMatch(/ổ hệ thống cho tới khi OneDrive\/iCloud giải phóng/);
    fireEvent.click(opt.querySelector('input')!); await settle();
    expect(screen.getByTestId('org-plan').textContent).toMatch(/44/);
    fireEvent.click(screen.getByTestId('org-start')); await settle();
    expect(screen.getByTestId('org-done').textContent).toMatch(/Đã chép 42 file/);
    expect(screen.queryByTestId('org-skipped-cloud')).toBeNull();
    expect(screen.getByTestId('org-issues').textContent).toMatch(/1 chưa tải về máy/);
    expect(groupSummary([{ code: 'cloud_not_downloaded', count: 250 }, { code: 'io', count: 3 }])).toBe('250 chưa tải về máy; 3 lỗi đọc/ghi');
  });

  it('không đủ chỗ để tải: khoá lựa chọn "Tải về rồi chép" và nói lý do', async () => {
    const api = createMockApi(); await api.organizedCreate('/Volumes/lowspace/Fonts');
    render(<OrganizedPanel api={api} />); await settle();
    fireEvent.click(screen.getByTestId('org-open-import')); await settle();
    expect((screen.getByTestId('org-download-opt').querySelector('input') as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByTestId('org-dl-nospace').textContent).toMatch(/Không đủ chỗ trống để tải/);
  });
});
