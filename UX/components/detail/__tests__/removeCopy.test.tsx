import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CmdError } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { RemoveCopyButton, removeCopyError } from '../RemoveCopy';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const face = { hashHex: 'copy', faceIndex: 0 };

describe('G7b — Gỡ bản sao đổi tên', () => {
  beforeEach(() => useNoticeStore.setState({ notices: [] }));
  afterEach(cleanup);

  it('nút → xác nhận (Thùng rác, không xoá vĩnh viễn) → gỡ → thông báo có Hoàn tác (lấy từ Thùng rác, không tự bật lại)', async () => {
    const api = createMockApi();
    const rm = vi.spyOn(api, 'removeDerivedCopy').mockResolvedValue({ removalId: 5, trashed: true, wasActive: true });
    const rs = vi.spyOn(api, 'restoreDerivedCopy');
    render(<RemoveCopyButton api={api} face={face} name="Oswald VF Regular" />);
    fireEvent.click(screen.getByTestId('remove-copy'));
    expect(screen.getByTestId('remove-copy-body').textContent).toMatch(/Thùng rác.*không xoá vĩnh viễn.*tắt nó trước/);
    fireEvent.click(screen.getByTestId('remove-copy-go')); await settle();
    expect(rm).toHaveBeenCalledWith(face);
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.text).toMatch(/Đã tắt và gỡ bản sao «Oswald VF Regular»/); expect(n.action?.label).toBe('Hoàn tác');
    act(() => n.action!.run()); await settle();
    expect(rs).toHaveBeenCalledWith(5);
    expect(useNoticeStore.getState().notices.at(-1)!.text).toMatch(/lấy lại bản sao.*từ Thùng rác/); expect(useNoticeStore.getState().notices.at(-1)!.text).toMatch(/chưa được bật lại/);
  });

  it('mã lỗi → câu dễ hiểu; gỡ lỗi thì bản sao còn nguyên và báo lỗi', async () => {
    expect(removeCopyError(new CmdError('trash_failed', 'x'))).toMatch(/Bản sao vẫn còn nguyên/);
    expect(removeCopyError(new CmdError('deactivate_failed', 'x'))).toMatch(/tắt nó rồi thử lại/);
    const api = createMockApi(); vi.spyOn(api, 'removeDerivedCopy').mockRejectedValue(new CmdError('not_derived', 'x'));
    render(<RemoveCopyButton api={api} face={face} name="X" />);
    fireEvent.click(screen.getByTestId('remove-copy')); fireEvent.click(screen.getByTestId('remove-copy-go')); await settle();
    expect(useNoticeStore.getState().notices.at(-1)).toMatchObject({ kind: 'error' });
  });
});
