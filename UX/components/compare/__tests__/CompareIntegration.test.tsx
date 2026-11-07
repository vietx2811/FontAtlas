import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { FaceActionsProvider } from '../../faces/FaceActions';
import { useNoticeStore } from '../../../store/notices';
import { useSelectionStore } from '../../../store/selection';
import { useUiStore } from '../../../store/ui';
import { Library } from '../../main/Library';
import { TopBar } from '../../main/TopBar';
import { ComparePage } from '../ComparePage';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const cells = () => screen.getAllByRole('gridcell').filter(c => c.hasAttribute('data-face-id'));

/** Như App.tsx: thư viện giữ mount (ẩn) khi đang Compare. */
function Shell({ api }: { api: ReturnType<typeof createMockApi> }) {
  const compare = useUiStore(s => s.compare);
  return <FaceActionsProvider api={api}>
    <div style={{ display: compare ? 'none' : 'contents' }}><TopBar api={api} /><Library api={api} width={1300} height={800} /></div>
    {compare && <ComparePage api={api} faces={compare} />}
  </FaceActionsProvider>;
}

describe('Compare over a mounted library', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useSelectionStore.getState().clear(); useNoticeStore.setState({ notices: [] }); });

  it('Esc closes Compare but keeps the library selection; the next Esc clears it', async () => {
    const api = createMockApi();
    render(<Shell api={api} />); await settle();
    fireEvent.click(cells()[0]); fireEvent.click(cells()[1], { metaKey: true });
    expect(useSelectionStore.getState().rows.size).toBe(2);
    fireEvent.click(screen.getByRole('button', { name: 'Compare' }));
    await screen.findByRole('table');
    fireEvent.keyDown(document.body, { key: 'Escape' });
    await settle();
    expect(useUiStore.getState().compare).toBeNull();
    expect(useSelectionStore.getState().rows.size).toBe(2);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useSelectionStore.getState().rows.size).toBe(0);
  });

  it('⌘A inside Compare does not select the hidden library', async () => {
    const api = createMockApi();
    render(<Shell api={api} />); await settle();
    fireEvent.click(cells()[0]); fireEvent.click(cells()[1], { metaKey: true });
    fireEvent.click(screen.getByRole('button', { name: 'Compare' })); await screen.findByRole('table');
    fireEvent.keyDown(document.body, { key: 'a', metaKey: true });
    await settle();
    expect(useSelectionStore.getState().rows.size).toBe(2);
  });

  it('⇧P / ⇧C are ignored while Compare is open', async () => {
    const api = createMockApi();
    render(<Shell api={api} />); await settle();
    fireEvent.click(cells()[0]); fireEvent.click(cells()[1], { metaKey: true });
    fireEvent.click(screen.getByRole('button', { name: 'Compare' })); await screen.findByRole('table');
    const tab = useUiStore.getState().panelTab;
    fireEvent.keyDown(document.body, { key: 'P', shiftKey: true });
    fireEvent.keyDown(document.body, { key: 'C', shiftKey: true });
    expect(useUiStore.getState().panelTab).toBe(tab);
    expect(useUiStore.getState().compare).toHaveLength(2);
  });
});
