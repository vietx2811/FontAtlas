import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { Api, SourceDto } from '../../../api';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { moveItem, mergeOrder, SourcePriority } from '../SourcePriority';

const initial = useUiStore.getState();
beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
const setup = async (api: Api = createMockApi()) => {
  const sources = await api.listSources();
  render(<SourcePriority api={api} sources={sources} />);
  await waitFor(() => expect(screen.getAllByTestId('prio-row')).toHaveLength(3));
  return { api, sources };
};
const names = () => screen.getAllByTestId('prio-row').map(r => r.querySelector('strong')!.textContent);

describe('mergeOrder / moveItem', () => {
  const s = (ids: number[]) => ids.map(id => ({ id }) as SourceDto);
  it('puts new sources last and drops ids with no source', () => { expect(mergeOrder([3, 9, 1], s([1, 2, 3]))).toEqual([3, 1, 2]); expect(mergeOrder([], s([1, 2]))).toEqual([1, 2]); });
  it('moves without mutating and ignores out-of-range', () => { const l = [1, 2, 3]; expect(moveItem(l, 0, 2)).toEqual([2, 3, 1]); expect(l).toEqual([1, 2, 3]); expect(moveItem(l, 0, 5)).toBe(l); expect(moveItem(l, 1, 1)).toBe(l); });
});

describe('SourcePriority', () => {
  it('lists sources in priority order with a position each, and the explanation', async () => {
    await setup();
    expect(names()).toEqual(['Fonts', 'Dự án khách', 'Ổ ngoài']);
    expect(screen.getAllByText(/^Ưu tiên \d$/).map(x => x.textContent)).toEqual(['Ưu tiên 1', 'Ưu tiên 2', 'Ưu tiên 3']);
    expect(screen.getByText(/bản ở nguồn đứng trên được giữ/)).toBeTruthy();
  });

  it('the down button sends the FULL list, shows the new order, announces it, and refreshes the library', async () => {
    const api = createMockApi(); const spy = vi.spyOn(api, 'setSourcePriority');
    await setup(api);
    fireEvent.click(screen.getByRole('button', { name: 'Đưa Fonts xuống' }));
    await waitFor(() => expect(names()).toEqual(['Dự án khách', 'Fonts', 'Ổ ngoài']));
    expect(spy).toHaveBeenCalledWith([2, 1, 3]);
    expect(await api.sourcePriority()).toEqual([2, 1, 3]);
    await waitFor(() => expect(screen.getByRole('status').textContent).toBe('Đã lưu thứ tự ưu tiên'));
    expect(useUiStore.getState().libraryRev).toBeGreaterThan(0);
  });

  it('the grip moves with the arrow keys; the first "up" and last "down" are inert', async () => {
    const api = createMockApi(); const spy = vi.spyOn(api, 'setSourcePriority');
    await setup(api);
    fireEvent.keyDown(screen.getByRole('button', { name: 'Kéo Ổ ngoài để đổi thứ tự' }), { key: 'ArrowUp' });
    await waitFor(() => expect(names()).toEqual(['Fonts', 'Ổ ngoài', 'Dự án khách']));
    expect(spy).toHaveBeenLastCalledWith([1, 3, 2]);
    spy.mockClear();
    fireEvent.click(screen.getByRole('button', { name: 'Đưa Fonts lên' }));
    fireEvent.keyDown(screen.getByRole('button', { name: 'Kéo Dự án khách để đổi thứ tự' }), { key: 'ArrowDown' });
    expect(spy).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Đưa Fonts lên' }).getAttribute('aria-disabled')).toBe('true');
  });

  it('dragging the grip with the pointer reorders on release', async () => {
    const api = createMockApi(); const spy = vi.spyOn(api, 'setSourcePriority');
    await setup(api);
    const rows = screen.getAllByTestId('prio-row');
    rows.forEach((r, i) => { r.getBoundingClientRect = () => ({ top: i * 40, height: 40, bottom: i * 40 + 40, left: 0, right: 100, width: 100, x: 0, y: i * 40, toJSON() {} }) as DOMRect; });
    const grip = within(rows[0]).getByRole('button', { name: /Kéo Fonts/ });
    // jsdom has no PointerEvent: a MouseEvent with the pointer type name is what React's pointer handlers read.
    const fire = (el: Element, type: string, init: MouseEventInit = {}) => act(() => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, ...init })); });
    fire(grip, 'pointerdown');
    fire(screen.getByRole('list'), 'pointermove', { clientY: 110 });
    fire(screen.getByRole('list'), 'pointerup');
    await waitFor(() => expect(spy).toHaveBeenCalledWith([2, 3, 1]));
    expect(names()).toEqual(['Dự án khách', 'Ổ ngoài', 'Fonts']);
  });

  it('a backend error puts the old order back and says so', async () => {
    const api = createMockApi(); vi.spyOn(api, 'setSourcePriority').mockRejectedValue(new Error('db locked'));
    await setup(api);
    fireEvent.click(screen.getByRole('button', { name: 'Đưa Fonts xuống' }));
    await waitFor(() => expect(names()).toEqual(['Fonts', 'Dự án khách', 'Ổ ngoài']));
    expect(useNoticeStore.getState().notices.at(-1)).toMatchObject({ kind: 'error', text: 'Không đổi được thứ tự: db locked' });
    expect(screen.getByRole('status').textContent).toBe('Không lưu được, đã trả về thứ tự cũ');
  });

  it('renders nothing with fewer than two sources (nothing to order)', async () => {
    const api = createMockApi(); const one = (await api.listSources()).slice(0, 1);
    const { container } = render(<SourcePriority api={api} sources={one} />);
    await act(async () => { await Promise.resolve(); });
    expect(container.querySelector('[data-testid=source-priority]')).toBeNull();
  });
});
