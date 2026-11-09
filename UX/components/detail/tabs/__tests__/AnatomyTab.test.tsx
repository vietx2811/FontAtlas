import { render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../../api/mock';
import type { AxisReadingDto, FaceDetailDto } from '../../../../api';
import { AXIS_META, AnatomyTab, guideLinesFrom, hxgStyle, percentileWord } from '../AnatomyTab';
import * as ink from '../../../ui/inkLines';

async function detail(): Promise<FaceDetailDto> {
  const api = createMockApi();
  const r = (await api.queryFaces({})).items.find(x => x.faceId % 3 === 0)!; // có trục không đo được
  return api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
}
const ax = (key: string, value: number | null, quality: AxisReadingDto['quality'] = 'measured'): AxisReadingDto => ({ key, value, rawValue: value, unit: key.endsWith("_em") ? "em" : "ratio", quality, reason: null, method: null, percentile: null });

describe('guideLinesFrom', () => {
  it('places baseline, x-height and cap lines from the measured ink', () => {
    const l = guideLinesFrom({ cap: 0.7, x: 0.5 }, 100, 110);
    expect(l.map(x => x.id)).toEqual(['cap', 'x', 'baseline']);
    expect(l.map(x => x.y)).toEqual([40, 60, 110]);
  });
  it('drops a line that could not be measured instead of guessing', () => {
    expect(guideLinesFrom({ cap: 0.7, x: null }, 100, 110).map(x => x.id)).toEqual(['cap', 'baseline']);
  });
});

describe('Hxg is drawn with exactly the measured face', () => {
  it('uses the own family, normal weight/style, no synthesis and default variation', () => {
    const st = hxgStyle('Fam');
    expect(st).toMatchObject({ fontFamily: '"Fam"', fontWeight: 'normal', fontStyle: 'normal', fontSynthesis: 'none', fontVariationSettings: 'normal' });
  });
});

describe('percentileWord', () => {
  it('turns a percentile into words, never a number', () => {
    expect([.05, .3, .5, .7, .95].map(percentileWord)).toEqual(['rất thấp', 'thấp', 'trung bình', 'cao', 'rất cao']);
  });
});

describe('AnatomyTab', () => {
  it('draws Hxg with cap / x / baseline guide lines', async () => {
    const d = await detail();
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue({ cap: d.axes.find(a => a.key === 'cap_height_em')!.rawValue as number, x: d.axes.find(a => a.key === 'x_height_em')!.rawValue as number });
    render(<AnatomyTab detail={d} family="X" />);
    const svg = screen.getByRole('img', { name: /Hxg/ });
    expect(svg.textContent).toContain('Hxg');
    await waitFor(() => expect(within(svg as unknown as HTMLElement).getAllByTestId('guide-line').length).toBe(3));
    expect(within(svg as unknown as HTMLElement).getAllByTestId('guide-line').map(l => l.getAttribute('data-line'))).toEqual(['cap', 'x', 'baseline']);
  });

  it('lists all 22 axes, each with its own icon', async () => {
    render(<AnatomyTab detail={await detail()} family="X" />);
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
    render(<AnatomyTab detail={d} family="X" />);
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
    render(<AnatomyTab detail={d} family="X" />);
    const rows = screen.getAllByTestId('axis-row');
    expect(rows[0].textContent).toContain('1,8');
    expect(rows[0].textContent).toContain('có chân');
    expect(rows[1].textContent).toContain('× cap-height');
    expect(rows[2].querySelector('.fa-axis-value')!.textContent).toContain('0');
    expect(rows[3].textContent).toContain('Chưa đo');
  });

  it('categorical axes read as words (a một/hai tầng), without a track', async () => {
    render(<AnatomyTab detail={await detail()} family="X" />);
    const a = screen.getAllByTestId('axis-row').find(r => r.getAttribute('data-axis') === 'a_storeys')!;
    expect(a.textContent).toMatch(/một tầng|hai tầng/);
    expect(a.querySelector('[data-track]')).toBeNull();
  });

  it('keeps percentile 0 and 1 distinct from missing, including estimated readings', async () => {
    const d = await detail();
    d.axes = [{...ax('contrast', 1), percentile: 0}, {...ax('width', 2, 'estimated'), percentile: 1}, {...ax('spacing', .2), percentile: null}];
    render(<AnatomyTab detail={d} family="X" />);
    const rows = screen.getAllByTestId('axis-row');
    expect(rows[0].textContent).toContain('P0');
    expect(rows[1].textContent).toContain('P100');
    expect(rows[1].textContent).toContain('Ước lượng');
    expect(rows[2].textContent).not.toMatch(/P[0-9]/);
  });

  it('never uses the pairing symbols ●◐◆○ or any score', async () => {
    const { container } = render(<AnatomyTab detail={await detail()} family="X" />);
    expect(container.textContent).not.toMatch(/[●◐◆○◑]/);
    expect(container.textContent).not.toMatch(/%/);
  });
});


describe('AnatomyTab guides vs the displayed glyphs', () => {
  afterEach(() => vi.restoreAllMocks());
  const capOf = async () => { const d = await detail(); return d.axes.find(a => a.key === 'cap_height_em')!.rawValue as number; };
  const xOf = async () => { const d = await detail(); return d.axes.find(a => a.key === 'x_height_em')!.rawValue as number; };

  it('matching: draws lines from the ink, no note', async () => {
    const [c, x] = [await capOf(), await xOf()];
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue({ cap: c, x });
    render(<AnatomyTab detail={await detail()} family="X" />);
    await waitFor(() => expect(screen.getAllByTestId('guide-line')).toHaveLength(3));
    expect(screen.queryByTestId('guides-note')).toBeNull();
  });
  it('mismatch > 1.5% em: still draws by the glyph and shows an i18n note with both values', async () => {
    const [c, x] = [await capOf(), await xOf()];
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue({ cap: c - 0.05, x });
    render(<AnatomyTab detail={await detail()} family="X" />);
    const note = await screen.findByTestId('guides-note');
    expect(note.textContent).toBe('Số đo analyzer khác hình hiển thị');
    const lines = screen.getAllByTestId('guide-line');
    const cap = lines.find(l => l.getAttribute('data-line') === 'cap')!;
    expect(Number(cap.getAttribute('y1'))).toBeCloseTo(112 - (c - 0.05) * 100, 5); // theo chữ, không theo analyzer
  });
  it('font cannot be loaded (TTC face ≥ 1 / no family): no guide lines at all, with a note', async () => {
    const d = await detail(); d.row.faceIndex = 1;
    render(<AnatomyTab detail={d} family={null} />);
    expect(screen.queryAllByTestId('guide-line')).toHaveLength(0);
    expect((await screen.findByTestId('guides-note')).textContent).toBe('Chưa nạp được font để minh hoạ đường kẻ');
  });
  it('measuring fails (loaded family but no ink): no lines and the unavailable note', async () => {
    vi.spyOn(ink.inkProbe, 'measure').mockResolvedValue(null);
    render(<AnatomyTab detail={await detail()} family="X" />);
    await screen.findByTestId('guides-note');
    expect(screen.queryAllByTestId('guide-line')).toHaveLength(0);
  });
});
