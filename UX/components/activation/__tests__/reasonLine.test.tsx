import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { ReasonLine, reasonText } from '../ReasonLine';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
afterEach(cleanup);

describe('B5 — vì sao đang bật', () => {
  it('reasonText: bốn loại lý do, có tên Case/app; chưa có tên thì hiện số', () => {
    const names = { 'c:7': 'Khách X', 'k:7': 'Case Y', 'a:7': 'Adobe Illustrator' };
    expect(reasonText({ kind: 'manual' }, names)).toBe('Bật thủ công');
    expect(reasonText({ kind: 'collection', collectionId: 7 }, names)).toBe('Từ «Khách X»');
    expect(reasonText({ kind: 'case_app', caseId: 7 }, names)).toBe('Mở cùng Adobe Illustrator · Case «Case Y»'); // cùng số 7: collection và Case là hai tên khác nhau
    expect(reasonText({ kind: 'auto', app: 'Photoshop' }, names)).toBe('Tự bật cho Photoshop');
    expect(reasonText({ kind: 'collection', collectionId: 9 }, names)).toBe('Từ «#9»');
  });

  it('một lý do: một dòng; nhiều lý do: lý do đầu (thủ công trước) + "+N"; không có lý do: không vẽ gì', async () => {
    const api = createMockApi(); const id = (await api.createCase('Khách X')).id;
    const { rerender, container } = render(<ReasonLine api={api} reasons={[{ kind: 'manual' }]} />);
    expect(screen.getByTestId('active-why').textContent).toBe('Bật thủ công'); expect(screen.queryByTestId('active-why-more')).toBeNull();
    rerender(<ReasonLine api={api} reasons={[{ kind: 'case_app', caseId: id }, { kind: 'auto', app: 'Photoshop' }, { kind: 'manual' }]} />); await settle();
    expect(screen.getByTestId('active-why').textContent).toMatch(/^Bật thủ công\+2$/);
    expect(screen.getByTestId('active-why-more').getAttribute('aria-label')).toBe('Tất cả 3 lý do đang bật');
    rerender(<ReasonLine api={api} reasons={[]} />); expect(container.querySelector('[data-testid=active-why]')).toBeNull();
  });

  it('lý do case: "Bật cả Case «tên»"; xếp sau thủ công, trước Dự án và app', async () => {
    expect(reasonText({ kind: 'case', caseId: 7 }, { 'k:7': 'Case Y' })).toBe('Bật cả Case «Case Y»');
    expect(reasonText({ kind: 'case', caseId: 9 }, {})).toBe('Bật cả Case «#9»');
    const api = createMockApi();
    render(<ReasonLine api={api} reasons={[{ kind: 'auto', app: 'Photoshop' }, { kind: 'collection', collectionId: 3 }, { kind: 'case', caseId: 5 }]} reasonNames={[{ collectionName: null, appNames: [] }, { collectionName: 'Dự án Z', appNames: [] }, { collectionName: 'Case Q', appNames: [] }]} />);
    expect(screen.getByTestId('active-why').textContent).toMatch(/^Bật cả Case «Case Q»\+2$/);
  });

  it('tên backend (reasonNames) dùng thẳng, không gọi lẻ; collection trùng số với Case không lấy nhầm tên Case', async () => {
    const api = createMockApi(); const caseId = (await api.createCase('Tên của Case')).id;
    const getCase = vi.spyOn(api, 'getCase'); const list = vi.spyOn(api, 'listCollections');
    // lý do collection có cùng số với Case: tên đúng là tên của backend
    render(<ReasonLine api={api} reasons={[{ kind: 'collection', collectionId: caseId }]} reasonNames={[{ collectionName: 'Dự án trùng số', appNames: [] }]} />);
    await settle();
    expect(screen.getByTestId('active-why').textContent).toBe('Từ «Dự án trùng số»');
    expect(getCase).not.toHaveBeenCalled(); expect(list).not.toHaveBeenCalled();
    cleanup();
    // không có tên backend: đường dự phòng tra collection bằng danh sách collection, KHÔNG qua getCase
    const col = await api.createCollection('Dự án tra lẻ', null);
    render(<ReasonLine api={api} reasons={[{ kind: 'collection', collectionId: col.id }]} />); await settle();
    expect(getCase).not.toHaveBeenCalled();
    expect(screen.getByTestId('active-why').textContent).toBe('Từ «Dự án tra lẻ»');
  });
});
