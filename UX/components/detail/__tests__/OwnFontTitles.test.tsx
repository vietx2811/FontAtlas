import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { makeFontUrl } from '../../../features/library-grid/api';
import { FaceActionsProvider } from '../../faces/FaceActions';
import { faceKey, useUiStore } from '../../../store/ui';
import { DetailPanel } from '../DetailPanel';
import { PairTab } from '../tabs/PairTab';
import { SimilarTab } from '../tabs/SimilarTab';

// jsdom không có FontFace/document.fonts: dựng bản giả nạp được ngay, để đi qua FontFaceLoader thật.
class FakeFace { constructor(public family: string, public src: string) {} load() { return Promise.resolve(this); } }
const added = new Set<FakeFace>();
const initial = useUiStore.getState();
/** Mock không có URL font → cho phép nạp như bản native. */
const mockApi = () => Object.assign(createMockApi(), { fontUrl: makeFontUrl(n => `/font/${n}`) });
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('font names rendered in the face itself', () => {
  beforeEach(() => {
    useUiStore.setState(initial, true);
    vi.stubGlobal('FontFace', FakeFace);
    Object.defineProperty(document, 'fonts', { configurable: true, value: { add: (f: FakeFace) => added.add(f), delete: (f: FakeFace) => added.delete(f) } });
  });
  afterEach(() => { vi.unstubAllGlobals(); added.clear(); });

  it('the panel heading and every style in the dropdown use their own face; the UI font is the fallback', async () => {
    const api = mockApi();
    const r = (await api.queryFaces({ text: 'Public Sans', limit: 100 })).items.find(x => x.subfamily === 'Regular')!;
    render(<DetailPanel api={api} />);
    act(() => useUiStore.getState().select(faceKey(r)));
    await settle();
    const h = screen.getByRole('heading', { level: 2 });
    expect(h.style.fontFamily).toContain(`fa-${r.hashHex}-0`);
    expect(h.style.fontFamily).toContain('var(--font-ui)');
    expect(h.dataset.ownFont).toBe(`fa-${r.hashHex}-0`);
    fireEvent.click(screen.getByRole('button', { name: 'Kiểu chữ' }));
    await settle();
    const opts = screen.getAllByRole('option');
    expect(opts.length).toBeGreaterThan(1);
    for (const o of opts) expect((o.querySelector('.lbl') as HTMLElement).dataset.ownFont).toMatch(/^fa-[0-9a-f]{64}-0$/);
  });

  it('Similar results title each family with its representative face and open a one-font menu on right-click', async () => {
    const api = mockApi();
    const base = (await api.queryFaces({ text: 'Roboto', limit: 1 })).items[0];
    const d = await api.getFace({ hashHex: base.hashHex, faceIndex: 0 }, 'vi');
    render(<FaceActionsProvider api={api}><SimilarTab api={api} detail={d} /></FaceActionsProvider>);
    const hits = await screen.findAllByTestId('similar-hit');
    await settle();
    const names = hits.map(h => h.querySelector('.fa-sim-name') as HTMLElement);
    names.forEach(n => { expect(n.style.fontFamily).toMatch(/fa-[0-9a-f]{64}-0/); expect(n.textContent).toBeTruthy(); });
    fireEvent.contextMenu(hits[0], { clientX: 30, clientY: 30 });
    expect(screen.getByRole('menuitem', { name: 'Mở chi tiết' })).toBeInTheDocument();
    expect(screen.queryByText(/font đã chọn/)).toBeNull();
    fireEvent.click(screen.getByRole('menuitem', { name: 'Tìm tương tự' }));
    expect(useUiStore.getState().panelTab).toBe('sim');
    expect(useUiStore.getState().selectedId).toBeTruthy();
  });

  it('Pairing candidate name uses the candidate face, and its card has the one-font menu', async () => {
    const api = mockApi();
    const base = (await api.queryFaces({ text: 'Playfair', limit: 1 })).items[0];
    const d = await api.getFace({ hashHex: base.hashHex, faceIndex: 0 }, 'vi');
    const { container } = render(<FaceActionsProvider api={api}><PairTab api={api} detail={d} css={{}} /></FaceActionsProvider>);
    await waitFor(() => expect(container.querySelector('.fa-pair-title')).toBeTruthy());
    await settle();
    expect((container.querySelector('.fa-pair-title') as HTMLElement).dataset.ownFont).toMatch(/^fa-[0-9a-f]{64}-0$/);
    fireEvent.contextMenu(container.querySelector('.fa-pair-card')!, { clientX: 20, clientY: 20 });
    expect(screen.getByRole('menuitem', { name: 'Ghép cặp' })).toBeInTheDocument();
  });
});
