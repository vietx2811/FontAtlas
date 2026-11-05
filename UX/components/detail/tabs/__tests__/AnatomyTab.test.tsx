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
const ax = (key: string, value: number | null, quality: AxisReadingDto['quality'] = 'measured'): AxisReadingDto => ({ key, value, quality, reason: null, method: null, percentile: null });

describe('glyphLines', () => {
  it('places baseline, x-height and cap lines from the readings', () => {
    const l = glyphLines([ax('x_height', 0.5), ax('cap_height', 0.7)], 100, 110);
    expect(l.map(x => x.id)).toEqual(['cap', 'x', 'baseline']);
    expect(l.map(x => x.y)).toEqual([40, 60, 110]);
  });
  it('drops a line whose axis is missing or failed instead of guessing', () => {
    expect(glyphLines([ax('x_height', null, 'failed'), ax('cap_height', 0.7)], 100, 110).map(x => x.id)).toEqual(['cap', 'baseline']);
    expect(glyphLines([], 100, 110).map(x => x.id)).toEqual(['baseline']);
  });
});

describe('percentileWord', () => {
  it('turns a percentile into words, never a number', () => {
    expect([5, 30, 50, 70, 95].map(percentileWord)).toEqual(['rất thấp', 'thấp', 'trung bình', 'cao', 'rất cao']);
  });
});

describe('AnatomyTab', () => {
  it('draws Hxg with cap / x / baseline guide lines', async () => {
    render(<AnatomyTab detail={await detail()} css={{ fontFamily: 'X' }} />);
    const svg = screen.getByRole('img', { name: /Hxg/ });
    expect(svg.textContent).toContain('Hxg');
    expect(within(svg as unknown as HTMLElement).getAllByTestId('guide-line').map(l => l.getAttribute('data-line'))).toEqual(['cap', 'x', 'baseline']);
  });

  it('lists all 19 axes, each with its own icon', async () => {
    render(<AnatomyTab detail={await detail()} css={{}} />);
    const rows = screen.getAllByTestId('axis-row');
    expect(rows).toHaveLength(19);
    const icons = rows.map(r => r.querySelector('[data-icon]')!.getAttribute('data-icon'));
    expect(new Set(icons).size).toBe(19);
    expect(Object.keys(AXIS_META).length).toBeGreaterThanOrEqual(19);
  });

  it('measured axes show a marker at the percentile; estimated ones are dashed and labelled; failed ones give the reason', async () => {
    render(<AnatomyTab detail={await detail()} css={{}} />);
    const rows = screen.getAllByTestId('axis-row');
    const measured = rows.find(r => r.getAttribute('data-quality') === 'measured' && r.querySelector('[data-marker]'))!;
    expect((measured.querySelector('[data-marker]') as HTMLElement).style.left).toMatch(/^\d+%$/);
    expect(measured.querySelector('[data-track]')!.className).not.toContain('dashed');
    const est = rows.find(r => r.getAttribute('data-quality') === 'estimated' && r.querySelector('[data-marker]'))!;
    expect(est.querySelector('[data-track]')!.className).toContain('dashed');
    expect(within(est).getByText('ước lượng')).toBeTruthy();
    const failed = rows.find(r => r.getAttribute('data-quality') === 'failed')!;
    expect(failed.querySelector('[data-marker]')).toBeNull();
    expect(failed.textContent).toMatch(/Không đo được/);
    expect(failed.textContent).toMatch(/chữ thường Latin/);
  });

  it('categorical axes read as words (a một/hai tầng), without a track', async () => {
    render(<AnatomyTab detail={await detail()} css={{}} />);
    const a = screen.getAllByTestId('axis-row').find(r => r.getAttribute('data-axis') === 'a_storeys')!;
    expect(a.textContent).toMatch(/một tầng|hai tầng/);
    expect(a.querySelector('[data-track]')).toBeNull();
  });

  it('never uses the pairing symbols ●◐◆○ or any score', async () => {
    const { container } = render(<AnatomyTab detail={await detail()} css={{}} />);
    expect(container.textContent).not.toMatch(/[●◐◆○◑]/);
    expect(container.textContent).not.toMatch(/%/);
  });
});
