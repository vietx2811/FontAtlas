import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { createMockApi } from '../../../../api/mock';
import type { FaceDetailDto } from '../../../../api';
import { VI_CHARS, ViTab } from '../ViTab';

async function detailOf(pred: (r: { viMissing: number; faceId: number }) => boolean): Promise<FaceDetailDto> {
  const api = createMockApi();
  const r = (await api.queryFaces({})).items.find(pred)!;
  return api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
}

describe('ViTab', () => {
  it('has the 134 Vietnamese characters (67 lowercase + 67 uppercase)', () => {
    expect(VI_CHARS).toHaveLength(134);
    expect(new Set(VI_CHARS).size).toBe(134);
  });

  it('a complete font: X/134 bar, no missing cells', async () => {
    const d = await detailOf(r => r.viMissing === 0 && r.faceId % 4 !== 1);
    render(<ViTab detail={d} css={{}} />);
    expect(screen.getByRole('meter', { name: /Ký tự tiếng Việt/ }).getAttribute('aria-valuenow')).toBe('134');
    expect(screen.getByTestId('vi-count').textContent).toBe('134/134');
    expect(screen.queryAllByTestId('vi-cell-missing')).toHaveLength(0);
    expect(screen.getAllByTestId('vi-cell')).toHaveLength(134);
  });

  it('missing cells are marked with a dashed border AND the word "thiếu", not colour alone', async () => {
    const d = await detailOf(r => r.viMissing === 9);
    render(<ViTab detail={d} css={{}} />);
    expect(screen.getByTestId('vi-count').textContent).toBe('125/134');
    const miss = screen.getAllByTestId('vi-cell-missing');
    expect(miss).toHaveLength(9);
    for (const c of miss) { expect(within(c).getByText('thiếu')).toBeTruthy(); expect(c.getAttribute('data-missing')).toBe('true'); }
  });

  it('warns about stacked-marks overflow only when the font reports it', async () => {
    const over = await detailOf(r => r.faceId % 4 === 1);
    const { unmount } = render(<ViTab detail={over} css={{}} />);
    expect(screen.getByRole('note').textContent).toMatch(/nhô quá/);
    unmount();
    const ok = await detailOf(r => r.faceId % 4 === 2);
    render(<ViTab detail={ok} css={{}} />);
    expect(screen.queryByRole('note')).toBeNull();
  });

  it('shows the stacked-marks sample and never a percentage', async () => {
    const d = await detailOf(r => r.viMissing === 9);
    const { container } = render(<ViTab detail={d} css={{}} />);
    expect(screen.getByTestId('vi-stack').textContent).toContain('Ắ');
    expect(container.textContent).not.toMatch(/%/);
  });
});
