import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act } from '@testing-library/react';
import { CmdError } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { hoverFinderDrop, useFinderDropStore } from '../../../store/finderDrop';
import { importPathsIntoTarget } from '../importDrop';

const last = () => useNoticeStore.getState().notices.at(-1)!;
const settle = async () => { for (let i = 0; i < 4; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('G8: nhập file từ Finder vào Dự án/Case', () => {
  beforeEach(() => useNoticeStore.setState({ notices: [] }));

  it('báo tiến độ có nút Huỷ, rồi số font mới + lý do bỏ qua; Hoàn tác gỡ đúng phần vừa thêm', async () => {
    const api = createMockApi(); const col = await api.createCollection('Poster', null);
    const faces = (await api.queryFaces({ limit: 2 })).items.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
    await api.addToCollection(col.id, [faces[0]]); // đã có sẵn từ trước: Hoàn tác không được gỡ nó
    const seen: string[] = [];
    vi.spyOn(api, 'importPathsInto').mockImplementation(async (_p, _t, onProgress) => {
      onProgress?.({ phase: 'scanning', done: 1, total: 2, added: 0, skipped: 0 }); seen.push(useNoticeStore.getState().notices.map(n => n.text).join('|'));
      await api.addToCollection(col.id, [faces[1]]);
      return { added: [faces[1]], alreadyThere: 1, skipped: [{ path: '/x/readme.txt', reason: 'not_font' }], cancelled: false };
    });
    await importPathsIntoTarget(api, ['/x'], { kind: 'col', id: col.id, name: 'Poster' }); await settle();
    expect(seen[0]).toMatch(/Đang quét để nhập vào «Poster»: 1\/2/);
    expect(useNoticeStore.getState().notices.some(n => n.action?.label === 'Huỷ')).toBe(false); // tiến độ đã được gỡ khi xong
    expect(last().text).toMatch(/Đã nhập .* vào «Poster»/); expect(last().text).toMatch(/1 font đã có sẵn/); expect(last().text).toMatch(/1 không phải font/);
    act(() => last().action!.run()); await settle();
    const left = (await api.queryFaces({ collectionId: col.id, limit: 10 })).items;
    expect(left).toHaveLength(1); expect(left[0].hashHex).toBe(faces[0].hashHex);
  });

  it('không có gì mới: nói "đã có sẵn" / "không có font nào" và không có Hoàn tác', async () => {
    const api = createMockApi(); const col = await api.createCollection('Poster', null);
    vi.spyOn(api, 'importPathsInto').mockResolvedValueOnce({ added: [], alreadyThere: 3, skipped: [], cancelled: false });
    await importPathsIntoTarget(api, ['/x'], { kind: 'col', id: col.id, name: 'Poster' });
    expect(last().text).toMatch(/đã có trong «Poster»/); expect(last().action).toBeUndefined();
    vi.spyOn(api, 'importPathsInto').mockResolvedValueOnce({ added: [], alreadyThere: 0, skipped: [{ path: '/x/a', reason: 'cloud_not_downloaded' }], cancelled: false });
    await importPathsIntoTarget(api, ['/x/a'], { kind: 'col', id: col.id, name: 'Poster' });
    expect(last().kind).toBe('error'); expect(last().text).toMatch(/chưa tải về máy/);
  });

  it('huỷ, đang bận, đích không còn', async () => {
    const api = createMockApi(); const col = await api.createCollection('Poster', null);
    vi.spyOn(api, 'importPathsInto').mockResolvedValueOnce({ added: [], alreadyThere: 0, skipped: [], cancelled: true });
    await importPathsIntoTarget(api, ['/x'], { kind: 'col', id: col.id, name: 'Poster' }); expect(last().text).toMatch(/Đã huỷ nhập/);
    vi.spyOn(api, 'importPathsInto').mockRejectedValueOnce(new CmdError('busy', 'b'));
    await importPathsIntoTarget(api, ['/x'], { kind: 'col', id: col.id, name: 'Poster' }); expect(last().text).toMatch(/lượt khác/);
    vi.spyOn(api, 'importPathsInto').mockRejectedValueOnce(new CmdError('not_found', 'n'));
    await importPathsIntoTarget(api, ['/x'], { kind: 'case', id: 9, name: 'Menu' }); expect(last().text).toMatch(/«Case Menu» không còn/);
  });

  it('đích Finder sáng lên khi rê trên Dự án/Case và tắt khi rời', () => {
    document.body.innerHTML = '<div id="d" data-drop-col="7" data-drop-name="Menu" data-drop-here="true"></div><div id="a" data-drop-active="0"></div>';
    const el = (id: string) => document.getElementById(id);
    let cur: Element | null = el('d'); (document as unknown as { elementFromPoint: unknown }).elementFromPoint = () => cur;
    hoverFinderDrop({ x: 1, y: 1 });
    expect(useFinderDropStore.getState()).toMatchObject({ armed: true, over: { kind: 'col', id: 7, valid: true } }); // xem chính Dự án đó vẫn nhập được
    cur = el('a'); hoverFinderDrop({ x: 2, y: 2 });
    expect(useFinderDropStore.getState().over).toBeNull(); // "Đang bật" không nhận file từ Finder
    hoverFinderDrop(null);
    expect(useFinderDropStore.getState()).toMatchObject({ armed: false, over: null });
  });
});
