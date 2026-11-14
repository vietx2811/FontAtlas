import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { ReasonLine, reasonText } from '../ReasonLine';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
afterEach(cleanup);

describe('B5 — vì sao đang bật', () => {
  it('reasonText: bốn loại lý do, có tên Case/app; chưa có tên thì hiện số', () => {
    const names = { 'c:7': 'Khách X', 'a:7': 'Adobe Illustrator' };
    expect(reasonText({ kind: 'manual' }, names)).toBe('Bật thủ công');
    expect(reasonText({ kind: 'collection', collectionId: 7 }, names)).toBe('Từ «Khách X»');
    expect(reasonText({ kind: 'case_app', collectionId: 7 }, names)).toBe('Mở cùng Adobe Illustrator · Case «Khách X»');
    expect(reasonText({ kind: 'auto', app: 'Photoshop' }, names)).toBe('Tự bật cho Photoshop');
    expect(reasonText({ kind: 'collection', collectionId: 9 }, names)).toBe('Từ «#9»');
  });

  it('một lý do: một dòng; nhiều lý do: lý do đầu (thủ công trước) + "+N"; không có lý do: không vẽ gì', async () => {
    const api = createMockApi(); const id = (await api.createCase('Khách X')).id;
    const { rerender, container } = render(<ReasonLine api={api} reasons={[{ kind: 'manual' }]} />);
    expect(screen.getByTestId('active-why').textContent).toBe('Bật thủ công'); expect(screen.queryByTestId('active-why-more')).toBeNull();
    rerender(<ReasonLine api={api} reasons={[{ kind: 'case_app', collectionId: id }, { kind: 'auto', app: 'Photoshop' }, { kind: 'manual' }]} />); await settle();
    expect(screen.getByTestId('active-why').textContent).toMatch(/^Bật thủ công\+2$/);
    expect(screen.getByTestId('active-why-more').getAttribute('aria-label')).toBe('Tất cả 3 lý do đang bật');
    rerender(<ReasonLine api={api} reasons={[]} />); expect(container.querySelector('[data-testid=active-why]')).toBeNull();
  });
});
