import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { OVERLAY_MAX, useUiStore } from '../../../store/ui';
import { ComparePage } from '../ComparePage';

vi.mock('../../detail/useFontFamily', () => ({ useFontFamily: () => 'F' }));
const initial = useUiStore.getState();
const rows = mockRows();
const fk = (i: number) => ({ hashHex: rows[i].hashHex, faceIndex: rows[i].faceIndex });
const key = (i: number) => `${rows[i].hashHex}:${rows[i].faceIndex}`;
beforeEach(() => { useUiStore.setState(initial, true); });
function Host({ api }: { api: ReturnType<typeof createMockApi> }) {
  const faces = useUiStore(s => s.compare);
  return faces ? <ComparePage api={api} faces={faces} /> : null;
}
async function mount(n = 5) {
  useUiStore.getState().openCompare(Array.from({ length: n }, (_, i) => fk(i)));
  render(<Host api={createMockApi()} />); await screen.findByRole('table');
}
const layers = () => [...document.querySelectorAll('.cmp-ovl text')].map(e => e.getAttribute('data-style'));
const eye = (name: RegExp) => screen.getAllByRole('button', { name }) as HTMLElement[];

describe('H8a · Overlay is a toggled shared area', () => {
  it('is not a table row any more; hidden until the toolbar button is pressed', async () => {
    await mount();
    expect(document.querySelector('.cmp-ovl')).toBeNull();
    expect(document.querySelector('[data-row="specimen.overlay"]')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Overlay' }));
    expect(document.querySelector('.cmp-ovl')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Overlay' }).getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'Overlay' }));
    expect(document.querySelector('.cmp-ovl')).toBeNull();
  });
  it('turning it on selects the base + the next font; the base is drawn solid, the other as a line, with a legend naming both', async () => {
    await mount();
    fireEvent.click(screen.getByRole('button', { name: 'Overlay' }));
    expect(layers()).toEqual(['solid', 'line']);
    const legend = [...document.querySelectorAll('.cmp-ovl-legend li')].map(li => li.textContent);
    expect(legend).toHaveLength(2);
    expect(legend[0]).toContain('mốc');
  });
  it('the eye on a column header toggles that font in/out; the 5th is refused (max 4) and later fonts get dash then dot', async () => {
    await mount(6);
    fireEvent.click(screen.getByRole('button', { name: 'Overlay' }));
    const eyes = () => document.querySelectorAll<HTMLElement>('.ch button[aria-label^="Overlay ·"]');
    expect(eyes()).toHaveLength(6);
    fireEvent.click(eyes()[2]); fireEvent.click(eyes()[3]);
    expect(layers()).toEqual(['solid', 'line', 'dash', 'dot']);
    expect(useUiStore.getState().compareSession!.overlayIds).toHaveLength(OVERLAY_MAX);
    fireEvent.click(eyes()[4]);
    expect(useUiStore.getState().compareSession!.overlayIds).toHaveLength(OVERLAY_MAX);
    expect(eyes()[4].getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(eyes()[1]);
    expect(layers()).toEqual(['solid', 'line', 'dash']);
  });
  it('eyes only exist while Overlay is on; with nothing selected it shows a hint', async () => {
    await mount(3);
    expect(document.querySelectorAll('.ch button[aria-label^="Overlay ·"]')).toHaveLength(0);
    act(() => { useUiStore.getState().setCompareOverlay(true); });
    act(() => { for (const k of [...useUiStore.getState().compareSession!.overlayIds]) useUiStore.getState().toggleCompareOverlayFace(k); });
    expect(screen.getByText(/Chọn font bằng nút mắt/)).toBeTruthy();
    expect(layers()).toEqual([]);
    expect(within(document.querySelector('.cmp-ovl')!).queryByRole('img')).toBeNull();
  });
  it('overlay choices live in the session store (kept when leaving the page)', async () => {
    await mount(3);
    act(() => { useUiStore.getState().setCompareOverlay(true); useUiStore.getState().closeCompare(); });
    act(() => { useUiStore.getState().resumeCompare(); });
    await screen.findByRole('table');
    expect(document.querySelector('.cmp-ovl')).toBeTruthy();
    expect(useUiStore.getState().compareSession!.overlayIds).toEqual([key(0), key(1)]);
  });
});
