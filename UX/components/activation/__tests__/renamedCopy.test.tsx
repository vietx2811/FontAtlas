import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CmdError } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useDeriveStore } from '../../../store/deriveCopy';
import { useNameClashStore } from '../../../store/nameClash';
import { useNoticeStore } from '../../../store/notices';
import { ClashNote } from '../../ui/ClashNote';
import { DeriveCopyDialog } from '../DeriveCopyDialog';
import { copyDoneText, deriveErrorText, keepBoth } from '../renamedCopy';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('G7b — bản sao đổi tên', () => {
  beforeEach(() => { useNoticeStore.setState({ notices: [] }); useDeriveStore.setState({ target: null }); useNameClashStore.getState().set([{ vfFaceId: 7, staticFaceIds: [8], psNames: ['Oswald-Regular'], exact: true }]); });
  afterEach(cleanup);

  it('câu thông báo: tên mới, đã có từ trước, và ghi chú giấy phép hạn chế (không chặn)', () => {
    const face = { hashHex: 'aa', faceIndex: 0 };
    expect(copyDoneText({ face, path: '/x', created: true, licenseRestricted: false }, 'Oswald VF Regular')).toBe('Đã tạo bản sao đổi tên: «Oswald VF Regular».');
    expect(copyDoneText({ face, path: '/x', created: false, licenseRestricted: false }, '')).toBe('Bản sao đổi tên đã có sẵn.');
    expect(copyDoneText({ face, path: '/x', created: true, licenseRestricted: true }, '')).toMatch(/giấy phép.*hạn chế việc sửa đổi.*không chặn/i);
  });

  it('mã lỗi → câu dễ hiểu', () => {
    expect(deriveErrorText(new CmdError('unsupported_container', 'x'))).toMatch(/TTF\/OTF/);
    expect(deriveErrorText(new CmdError('not_downloaded', 'x'))).toMatch(/tải về máy/);
    expect(deriveErrorText(new Error('boom'))).toMatch(/boom/);
  });

  it('bấm dấu "!" của font VF trùng tên mở hộp thoại; Tạo bản sao gọi lệnh rồi báo; static không có hành động', async () => {
    const api = createMockApi();
    render(<><ClashNote faceId={7} face={{ hashHex: 'vv', faceIndex: 0 }} /><ClashNote faceId={8} face={{ hashHex: 'ss', faceIndex: 0 }} /><DeriveCopyDialog api={api} /></>);
    const marks = screen.getAllByTestId('name-clash-note');
    expect(marks[0].getAttribute('role')).toBe('button'); expect(marks[1].getAttribute('role')).toBe('img');
    fireEvent.click(marks[0]); await settle();
    expect(screen.getByText(/PostScript name/)).toBeTruthy();
    fireEvent.click(screen.getByTestId('copy-make')); await settle();
    expect(useNoticeStore.getState().notices.at(-1)!.text).toMatch(/Đã tạo bản sao đổi tên/);
    expect(screen.queryByTestId('copy-make')).toBeNull();
  });

  it('Giữ cả hai qua activation_keep_both: toast nêu tên bản sao + ghi chú giấy phép; lỗi tạo bản sao nói lý do và đã bật như cũ', async () => {
    const api = createMockApi(); const first = (await api.queryFaces({ limit: 1 })).items[0];
    const key = { hashHex: first.hashHex, faceIndex: first.faceIndex };
    const spy = vi.spyOn(api, 'activationKeepBoth').mockResolvedValue({ results: [{ hashHex: key.hashHex, requestedFaces: [key.faceIndex], outcome: { kind: 'activated' }, warning: 'renamed_copy_restricted' }] as never, changed: 1, needsConfirmation: 0, failed: 0 });
    await keepBoth(api, [key]);
    expect(spy).toHaveBeenCalledWith([key], null);
    const texts = useNoticeStore.getState().notices.map(n => n.text).join('|');
    expect(texts).toMatch(/Đã bật bản sao đổi tên «.* VF.*» để dùng cả hai/); expect(texts).toMatch(/hạn chế việc sửa đổi/);
    useNoticeStore.setState({ notices: [] });
    spy.mockResolvedValue({ results: [{ hashHex: key.hashHex, requestedFaces: [0], outcome: { kind: 'activated' }, warning: 'rename_failed:disk_full' }] as never, changed: 1, needsConfirmation: 0, failed: 0 });
    await keepBoth(api, [key]);
    const err = useNoticeStore.getState().notices.find(n => n.kind === 'error')!; expect(err.text).toMatch(/Hết chỗ trống.*bật như cũ/);
  });
});
