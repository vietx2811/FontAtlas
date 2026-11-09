import { fireEvent, render, screen } from '@testing-library/react';
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import type { CompareMetric } from '../../../api/compare';
import { MetricCell } from '../CompareCells';
import { TOOLTIP_DELAY_MS } from '../../ui/Tooltip';

const m = (value: number | null, o: Partial<CompareMetric> = {}): CompareMetric => ({ key: 'x', value, unit: 'em', quality: 'measured', reason: null, percentile: 97, ...o });
const bar = () => document.querySelector('.cmp-pbar');

describe('H8b · MetricCell bar', () => {
  it('draws the marker by the compared min–max and keeps the library percentile out of the cell (tooltip only)', async () => {
    const { container } = render(<MetricCell k="x_height_em" m={m(0.52)} base={m(0.5)} isBase={false} range={{ lo: 0.498, hi: 0.522 }} />);
    expect(Number((bar()!.firstElementChild as HTMLElement).style.left.replace('%', ''))).toBeGreaterThan(90);
    expect(container.querySelector('.cmp-pbar em')).toBeNull();
    expect(container.textContent).not.toMatch(/P97/);
    expect(bar()!.getAttribute('aria-label')).toBe('Percentile 97');
    fireEvent.pointerEnter(bar()!);
    await act(async () => { await new Promise(r => setTimeout(r, TOOLTIP_DELAY_MS + 60)); });
    expect(screen.getByRole('tooltip').textContent).toMatch(/Percentile 97/);
  });
  it('draws no bar without a range (single value / all equal)', () => {
    render(<MetricCell k="x_height_em" m={m(0.5)} base={m(0.5)} isBase range={null} />);
    expect(bar()).toBeNull();
  });
  it('a missing percentile still gets a bar with a neutral label', () => {
    render(<MetricCell k="x_height_em" m={m(0.5, { percentile: null })} base={m(0.5)} isBase range={{ lo: 0, hi: 1 }} />);
    expect(bar()!.getAttribute('aria-label')).toBe('Vị trí giữa các font đang so sánh');
  });
});
