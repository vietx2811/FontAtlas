import { act, fireEvent, render, renderHook, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActivationReportDto } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { setLocale } from '../../../i18n';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { RelocateDialog } from '../../library/RelocateDialog';
import { busyFaces, busyN, busyPart, pushBusyOnly } from '../busyNotice';
import { useActivationFlow } from '../useActivationFlow';

const rep = (kinds: string[], extra: Record<string, unknown> = {}): ActivationReportDto => ({ results: kinds.map((k, i) => ({ hashHex: `h${i}`, requestedFaces: [0], outcome: { kind: k } })) as never, changed: 0, needsConfirmation: 0, failed: 0, busy: 0, busyHashes: [], ...extra } as unknown as ActivationReportDto);
const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('font đang bận (sau Tắt hết / đổi scope / chuyển font)', () => {
  beforeEach(() => { setLocale('vi'); useNoticeStore.setState({ notices: [] }); });
  afterEach(() => setLocale('vi'));

  it('số và danh sách font cùng một nguồn (kết quả theo file); chỉ khi không có kết quả theo file mới dùng report.busy', () => {
    expect(busyN(rep(['deactivated', 'busy', 'busy']))).toBe(2);
    expect(busyN(rep(['busy', 'busy'], { busy: 9 }))).toBe(2); // khớp nút Thử lại, không lấy 9
    expect(busyN(rep([], { busy: 5 }))).toBe(5);
    expect(busyN(rep(['busy'], { busy: 3, busyHashes: ['h0', 'x1', 'x2'] }))).toBe(3); // busyHashes bổ sung font mà results không còn; số = số hash trong danh sách
    expect(busyFaces(rep(['busy'], { busyHashes: ['h0', 'x1'] }))).toEqual([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'x1', faceIndex: 0 }]);
    expect(busyFaces(rep(['busy', 'deactivated', 'busy']))).toEqual([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'h2', faceIndex: 0 }]);
    expect(busyPart(rep(['deactivated']))).toBeNull();
  });

  it('câu số ít/nhiều ở cả hai ngôn ngữ', () => {
    expect(busyPart(rep(['busy']))).toBe('1 font đang bận, sẽ thử lại');
    setLocale('en');
    expect(busyPart(rep(['busy']))).toBe('1 font busy, will retry');
    expect(busyPart(rep(['busy', 'busy']))).toBe('2 fonts busy, will retry');
  });

  it('pushBusyOnly: im lặng khi không bận (và không phải thử lại); bận → một thông báo + Thử lại; thử lại xong thì nói đã xong, không đẩy tóm tắt rỗng', () => {
    const retry = vi.fn();
    pushBusyOnly(rep(['deactivated']), retry); expect(useNoticeStore.getState().notices).toHaveLength(0);
    pushBusyOnly(rep(['busy', 'busy']), retry);
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.text).toBe('2 font đang bận, sẽ thử lại'); expect(n.action?.label).toBe('Thử lại');
    act(() => n.action!.run()); expect(retry).toHaveBeenCalledTimes(1);
    useNoticeStore.setState({ notices: [] });
    pushBusyOnly(rep(['deactivated']), retry, true);
    expect(useNoticeStore.getState().notices.map(x => x.text)).toEqual(['Đã xong: không còn font nào bận.']);
  });

  it('đổi scope gặp font bận → thông báo gộp (không báo từng file); Thử lại gọi lại set_scope với đúng font bận', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'activationSetScope').mockResolvedValueOnce(rep(['busy', 'busy'])).mockResolvedValue(rep(['activated']));
    const { result } = renderHook(() => useActivationFlow(api));
    await act(async () => { await result.current.setScope([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'h1', faceIndex: 0 }], 'persistent'); });
    const notices = useNoticeStore.getState().notices;
    expect(notices).toHaveLength(1); expect(notices[0].text).toMatch(/^2 font đang bận/);
    await act(async () => { notices[0].action!.run(); await new Promise(r => setTimeout(r, 0)); });
    expect(spy).toHaveBeenLastCalledWith([{ hashHex: 'h0', faceIndex: 0 }, { hashHex: 'h1', faceIndex: 0 }], 'persistent');
    expect(useNoticeStore.getState().notices.at(-1)!.text).toBe('Đã xong: không còn font nào bận.');
  });

  it('chuyển font: MỘT thông báo (xong + đang bận) với Hoàn tác và Thử lại; Thử lại chạy relocate cho đúng font bận và báo lỗi nếu hỏng', async () => {
    const api = createMockApi();
    useUiStore.setState({ active: new Map([['h0', [0]], ['h1', [0]], ['h2', [0]]]) });
    vi.spyOn(api, 'activationState').mockResolvedValue([{ hashHex: 'h0', faces: [0], state: 'active', scope: 'session', status: 'active', reasons: [], activePath: '/old/a.ttf', conflictPath: null, conflictPs: null, lastError: null }] as never);
    const reloc = vi.spyOn(api, 'activationRelocate').mockResolvedValueOnce(rep(['activated', 'busy', 'busy'])).mockRejectedValueOnce(new Error('boom'));
    render(<RelocateDialog api={api} preferUnder="/lib/Library" onClose={() => undefined} />);
    fireEvent.click(screen.getByTestId('org-reloc-run')); await settle();
    const ns = useNoticeStore.getState().notices; expect(ns).toHaveLength(1);
    expect(ns[0].text).toMatch(/Đã chuyển 1 font.*2 font đang bận, sẽ thử lại/);
    expect(ns[0].action?.label).toBe('Hoàn tác'); expect(ns[0].action2?.label).toBe('Thử lại');
    await act(async () => { ns[0].action2!.run(); await new Promise(r => setTimeout(r, 0)); });
    expect(reloc).toHaveBeenLastCalledWith([{ hashHex: 'h1', faceIndex: 0 }, { hashHex: 'h2', faceIndex: 0 }], '/lib/Library');
    expect(useNoticeStore.getState().notices.at(-1)).toMatchObject({ kind: 'error' }); // lỗi của lần thử lại không bị nuốt
    void within;
  });

  it('lỗi ném từ báo cáo của lần thử lại (refreshActive hỏng) vẫn thành thông báo lỗi, không phải unhandled rejection', async () => {
    const api = createMockApi();
    useUiStore.setState({ active: new Map([['h0', [0]]]) });
    vi.spyOn(api, 'activationRelocate').mockResolvedValueOnce(rep(['busy'])).mockResolvedValueOnce(rep(['activated']));
    vi.spyOn(api, 'activationState').mockResolvedValueOnce([] as never).mockResolvedValueOnce([] as never).mockRejectedValue(new Error('state down'));
    const unhandled = vi.fn(); process.on('unhandledRejection', unhandled);
    render(<RelocateDialog api={api} preferUnder="/lib" onClose={() => undefined} />);
    fireEvent.click(screen.getByTestId('org-reloc-run')); await settle();
    const n = useNoticeStore.getState().notices.at(-1)!;
    await act(async () => { n.action!.run(); await new Promise(r => setTimeout(r, 20)); });
    expect(useNoticeStore.getState().notices.at(-1)).toMatchObject({ kind: 'error' });
    expect(useNoticeStore.getState().notices.at(-1)!.text).toMatch(/state down/);
    process.off('unhandledRejection', unhandled); expect(unhandled).not.toHaveBeenCalled();
  });
});
