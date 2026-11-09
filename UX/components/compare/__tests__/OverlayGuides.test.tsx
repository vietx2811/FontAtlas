import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import * as ink from '../../ui/inkLines';
import { OverlayCell } from '../CompareCells';

vi.mock('../../detail/useFontFamily', () => ({ useFontFamily: () => 'F' }));
afterEach(() => vi.restoreAllMocks());
const faces = async () => { const api = createMockApi(); const d = await api.compareFaces(mockRows().slice(0, 2).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }))); return { api, d }; };

describe('Overlay x-height line follows the displayed base glyph', () => {
  it('draws the dashed line at the measured x ink, not the analyzer number', async () => {
    const { api, d } = await faces();
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue({ cap: 0.7, x: 0.4321 });
    render(<OverlayCell api={api} base={d.faces[0]} face={d.faces[1]} scale={1} />);
    const line = await screen.findByTestId('overlay-xh');
    expect(Number(line.getAttribute('y1'))).toBeCloseTo(66 - 0.4321 * 64, 5);
  });
  it('draws no x-height line when the base cannot be measured (no cross-font guess)', async () => {
    const { api, d } = await faces();
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue(null);
    render(<OverlayCell api={api} base={d.faces[0]} face={d.faces[1]} scale={1} />);
    await new Promise(r => setTimeout(r, 50));
    expect(screen.queryByTestId('overlay-xh')).toBeNull();
    expect(document.querySelector('.cmp-overlay .bl')).toBeTruthy(); // baseline luôn có
  });
});
