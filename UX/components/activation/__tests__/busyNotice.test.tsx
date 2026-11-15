import { act } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActivationReportDto } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { busyCount, busyFaces, noticeBusy } from '../busyNotice';

const rep = (kinds: string[], busy?: number): ActivationReportDto => ({ results: kinds.map((k, i) => ({ hashHex: `h${i}`, requestedFaces: [0], outcome: { kind: k } })) as never, changed: 0, needsConfirmation: 0, failed: 0, ...(busy === undefined ? {} : { busy }) } as ActivationReportDto);

describe('font đang bận (sau Tắt hết / đổi scope / chuyển font)', () => {
  beforeEach(() => useNoticeStore.setState({ notices: [] }));

  it('đếm: ưu tiên trường busy của backend, không có thì đếm kết quả busy theo file', () => {
    expect(busyCount(rep(['deactivated', 'busy', 'busy']))).toBe(2);
    expect(busyCount(rep(['deactivated', 'busy'], 5))).toBe(5);
    expect(busyFaces(rep(['busy', 'deactivated', 'busy']))).toEqual([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'h2', faceIndex: 0 }]);
  });

  it('không bận: im lặng', () => {
    expect(noticeBusy(rep(['deactivated']), () => undefined)).toBe(false);
    expect(useNoticeStore.getState().notices).toHaveLength(0);
  });

  it('bận: một thông báo gộp "N font đang bận, sẽ thử lại" có nút Thử lại chạy lại đúng những font bận', () => {
    const retry = vi.fn();
    expect(noticeBusy(rep(['busy', 'activated', 'busy']), retry)).toBe(true);
    const n = useNoticeStore.getState().notices.at(-1)!;
    expect(n.text).toMatch(/^2 font đang bận, sẽ thử lại/); expect(n.action?.label).toBe('Thử lại');
    act(() => n.action!.run());
    expect(retry).toHaveBeenCalledWith([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'h2', faceIndex: 0 }]);
  });

  it('đổi scope gặp font bận → thông báo gộp (không báo từng file); Thử lại gọi lại set_scope với đúng font bận', async () => {
    const { renderHook } = await import('@testing-library/react');
    const { useActivationFlow } = await import('../useActivationFlow');
    const api = createMockApi();
    const spy = vi.spyOn(api, 'activationSetScope').mockResolvedValueOnce(rep(['busy', 'busy'])).mockResolvedValue(rep(['activated']));
    const { result } = renderHook(() => useActivationFlow(api));
    await act(async () => { await result.current.setScope([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'h1', faceIndex: 0 }], 'persistent'); });
    const notices = useNoticeStore.getState().notices;
    expect(notices).toHaveLength(1); expect(notices[0].text).toMatch(/^2 font đang bận/);
    await act(async () => { notices[0].action!.run(); await new Promise(r => setTimeout(r, 0)); });
    expect(spy).toHaveBeenLastCalledWith([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'h1', faceIndex: 0 }], 'persistent');
  });
});
