import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BEAT_MS, ScanBlocks, usePrefersReducedMotion } from '../ScanBlocks';

function mockMatchMedia(reduced: boolean) {
  vi.stubGlobal('matchMedia', (q: string) => ({ matches: reduced && q.includes('prefers-reduced-motion'), media: q, addEventListener: () => {}, removeEventListener: () => {} }));
}
beforeEach(() => { vi.useFakeTimers(); mockMatchMedia(false); });
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });

describe('ScanBlocks', () => {
  it('draws 9 square blocks, each with one letter set in a system font; no preview frame, no grid', () => {
    const { container } = render(<ScanBlocks seed={1} scanning progress={0.3} done={false} />);
    const blocks = screen.getAllByTestId('scan-block');
    expect(blocks).toHaveLength(9);
    for (const b of blocks) {
      expect(b.textContent).toMatch(/^\S$/u);
      expect((b as HTMLElement).style.fontFamily).toMatch(/\S/);
    }
    expect(container.querySelector('[role=grid], [role=gridcell], img')).toBeNull();
  });

  it('turns the blocks while scanning and changes letter/font on every beat', () => {
    render(<ScanBlocks seed={1} scanning progress={0.3} done={false} />);
    const grid = screen.getByTestId('scan-blocks');
    expect(grid.getAttribute('data-spinning')).toBe('true');
    const snap = () => screen.getAllByTestId('scan-block').map(b => `${b.textContent}|${(b as HTMLElement).style.fontFamily}`).join();
    const a = snap();
    act(() => { vi.advanceTimersByTime(BEAT_MS + 10); });
    expect(snap()).not.toBe(a);
  });

  it('stands still when not scanning (no beats)', () => {
    render(<ScanBlocks seed={1} scanning={false} progress={0} done={false} />);
    expect(screen.getByTestId('scan-blocks').getAttribute('data-spinning')).toBe('false');
    const snap = () => screen.getAllByTestId('scan-block').map(b => b.textContent).join();
    const a = snap();
    act(() => { vi.advanceTimersByTime(BEAT_MS * 3); });
    expect(snap()).toBe(a);
  });

  it('respects prefers-reduced-motion: no rotation, a fade instead', () => {
    mockMatchMedia(true);
    render(<ScanBlocks seed={1} scanning progress={0.3} done={false} />);
    const grid = screen.getByTestId('scan-blocks');
    expect(grid.getAttribute('data-motion')).toBe('reduced');
    expect(grid.getAttribute('data-spinning')).toBe('false');
    act(() => { vi.advanceTimersByTime(BEAT_MS + 10); });
    expect(screen.getAllByTestId('scan-block').some(b => b.getAttribute('data-fade') === 'true')).toBe(true);
  });

  it('only a progress bar under the blocks while scanning, then a green check circle when done', () => {
    const { rerender } = render(<ScanBlocks seed={1} scanning progress={0.4} done={false} />);
    const bar = screen.getByRole('progressbar');
    expect(bar.getAttribute('aria-valuenow')).toBe('40');
    expect(screen.queryByRole('img', { name: 'Đã quét xong' })).toBeNull();
    rerender(<ScanBlocks seed={1} scanning={false} progress={1} done />);
    expect(screen.getByRole('img', { name: 'Đã quét xong' })).toBeTruthy();
    expect(screen.getByTestId('scan-blocks').getAttribute('data-spinning')).toBe('false');
  });

  it('an unknown total shows an indeterminate bar', () => {
    render(<ScanBlocks seed={1} scanning progress={null} done={false} />);
    expect(screen.getByRole('progressbar').hasAttribute('aria-valuenow')).toBe(false);
  });
});

describe('usePrefersReducedMotion', () => {
  it('reads the media query', () => {
    mockMatchMedia(true);
    let v = false;
    function T() { v = usePrefersReducedMotion(); return null; }
    render(<T />);
    expect(v).toBe(true);
  });
  it('is false when matchMedia is missing (old webview)', () => {
    vi.stubGlobal('matchMedia', undefined);
    let v = true;
    function T() { v = usePrefersReducedMotion(); return null; }
    render(<T />);
    expect(v).toBe(false);
  });
});
