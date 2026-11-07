import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { createMockApi } from '../../../../api/mock';
import type { AxisReadingDto, FaceDetailDto } from '../../../../api';
import { AXIS_META, AnatomyTab, glyphLines, percentileWord } from '../AnatomyTab';

async function detail(): Promise<FaceDetailDto> {
  const api = createMockApi();
  const r = (await api.queryFaces({})).items.find(x => x.faceId % 3 === 0)!; // có trục không đo được
  return api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
}
const ax = (key: string, value: number | null, quality: AxisReadingDto['quality'] = 'measured'): AxisReadingDto => ({ key, value, rawValue: value, unit: key.endsWith("_em") ? "em" : "ratio", quality, reason: null, method: null, percentile: null });

describe('glyphLines', () => {
  it('places baseline, x-height and cap lines from the readings', () => {
    const l = glyphLines([ax('x_height_em', 0.5), ax('cap_height_em', 0.7)], 100, 110);
    expect(l.map(x => x.id)).toEqual(['cap', 'x', 'baseline']);
    expect(l.map(x => x.y)).toEqual([40, 60, 110]);
  });
  it('drops a line whose axis is missing or failed instead of guessing', () => {
    expect(glyphLines([ax('x_height_em', null, 'failed'), ax('cap_height_em', 0.7)], 100, 110).map(x => x.id)).toEqual(['cap', 'baseline']);
    expect(glyphLines([], 100, 110).map(x => x.id)).toEqual(['baseline']);
  });
});

describe('percentileWord', () => {
  it('turns a percentile into words, never a number', () => {
    expect([.05, .3, .5, .7, .95].map(percentileWord)).toEqual(['rất thấp', 'thấp', 'trung bình', 'cao', 'rất cao']);
  });
});

describe('AnatomyTab', () => {
  it('draws Hxg with cap / x / baseline guide lines', async () => {
    render(<AnatomyTab detail={await detail()} css={{ fontFamily: 'X' }} />);
    const svg = screen.getByRole('img', { name: /Hxg/ });
    expect(svg.textContent).toContain('Hxg');
    expect(within(svg as unknown as HTMLElement).getAllByTestId('guide-line').map(l => l.getAttribute('data-line'))).toEqual(['cap', 'x', 'baseline']);
  });

  it('lists all 22 axes, each with its own icon', async () => {
    render(<AnatomyTab detail={await detail()} css={{}} />);
    const rows = screen.getAllByTestId('axis-row');
    expect(rows).toHaveLength(22);
    const icons = rows.map(r => r.querySelector('[data-icon]')!.getAttribute('data-icon'));
    expect(new Set(icons).size).toBe(22);
    expect(Object.keys(AXIS_META).length).toBeGreaterThanOrEqual(22);
  });

  it('shows numbers with units, estimated status and failure reasons', async () => {
    const d = await detail();
    d.axes = [ax('slant_deg', -12), {...ax('x_height_em', .523, 'estimated'), percentile: .94}, ax('spacing', null, 'failed')];
    d.axes[0].unit = 'deg'; d.axes[2].reason = 'no_latin_lowercase';
    render(<AnatomyTab detail={d} css={{}} />);
    const rows = screen.getAllByTestId('axis-row');
    expect(rows[0].textContent).toContain('-12');
    expect(rows[0].textContent).toContain('°');
    expect(rows[1].textContent).toContain('0,523');
    expect(rows[1].textContent).toContain('em');
    expect(rows[1].textContent).toContain('Ước lượng');
    expect(rows[1].textContent).toContain('P94');
    expect(rows[2].textContent).toContain('Không đo được');
    expect(rows[2].textContent).toContain('chữ thường Latin');
  });

  it('keeps raw serif values, units from caps fallback, zero and missing distinct', async () => {
    const d = await detail();
    d.axes = [{...ax('serif_foot', 1.8), value: 'serif'}, {...ax('stem', .2), unit: 'cap_height_ratio'}, ax('slant_deg', 0), ax('unknown', null)];
    render(<AnatomyTab detail={d} css={{}} />);
    const rows = screen.getAllByTestId('axis-row');
    expect(rows[0].textContent).toContain('1,8');
    expect(rows[0].textContent).toContain('có chân');
    expect(rows[1].textContent).toContain('× cap-height');
    expect(rows[2].querySelector('.fa-axis-value')!.textContent).toContain('0');
    expect(rows[3].textContent).toContain('Chưa đo');
  });

  it('categorical axes read as words (a một/hai tầng), without a track', async () => {
    render(<AnatomyTab detail={await detail()} css={{}} />);
    const a = screen.getAllByTestId('axis-row').find(r => r.getAttribute('data-axis') === 'a_storeys')!;
    expect(a.textContent).toMatch(/một tầng|hai tầng/);
    expect(a.querySelector('[data-track]')).toBeNull();
  });

  it('keeps percentile 0 and 1 distinct from missing, including estimated readings', async () => {
    const d = await detail();
    d.axes = [{...ax('contrast', 1), percentile: 0}, {...ax('width', 2, 'estimated'), percentile: 1}, {...ax('spacing', .2), percentile: null}];
    render(<AnatomyTab detail={d} css={{}} />);
    const rows = screen.getAllByTestId('axis-row');
    expect(rows[0].textContent).toContain('P0');
    expect(rows[1].textContent).toContain('P100');
    expect(rows[1].textContent).toContain('Ước lượng');
    expect(rows[2].textContent).not.toMatch(/P[0-9]/);
  });

  it('never uses the pairing symbols ●◐◆○ or any score', async () => {
    const { container } = render(<AnatomyTab detail={await detail()} css={{}} />);
    expect(container.textContent).not.toMatch(/[●◐◆○◑]/);
    expect(container.textContent).not.toMatch(/%/);
  });
});
