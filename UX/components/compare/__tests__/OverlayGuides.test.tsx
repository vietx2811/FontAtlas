import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import * as ink from '../../ui/inkLines';
import { OverlayPanel } from '../CompareCells';

vi.mock('../../detail/useFontFamily', () => ({ useFontFamily: () => 'F' }));
afterEach(() => vi.restoreAllMocks());
const faces = async () => { const api = createMockApi(); const d = await api.compareFaces(mockRows().slice(0, 3).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }))); return { api, d }; };

describe('Overlay panel x-height line follows the displayed primary glyph', () => {
  it('draws the dashed line at the measured x ink, not the analyzer number (exactly one line for the shared panel)', async () => {
    const { api, d } = await faces();
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue({ cap: 0.7, x: 0.4321 });
    render(<OverlayPanel api={api} base={d.faces[0]} selected={[d.faces[0], d.faces[1]]} scale={() => 1} />);
    const line = await screen.findByTestId('overlay-xh');
    expect(Number(line.getAttribute('y1'))).toBeCloseTo(80 - 0.4321 * 72, 5);
    expect(screen.getAllByTestId('overlay-xh')).toHaveLength(1);
  });
  it('draws no x-height line when the primary cannot be measured (no cross-font guess); baseline always there', async () => {
    const { api, d } = await faces();
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue(null);
    render(<OverlayPanel api={api} base={d.faces[0]} selected={[d.faces[0], d.faces[1]]} scale={() => 1} />);
    await new Promise(r => setTimeout(r, 50));
    expect(screen.queryByTestId('overlay-xh')).toBeNull();
    expect(document.querySelector('.cmp-ovl .bl')).toBeTruthy();
  });
});
