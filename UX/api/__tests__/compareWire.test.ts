import { describe, expect, it } from 'vitest';
import wire from '../../bindings/fixtures/compare_two.json';
import type { CompareDto as WireDto } from '../../bindings/CompareDto';
import { COMPARE_METRICS } from '../compare';
import { fromWire } from '../compareWire';

const dto = wire as unknown as WireDto;

describe('fromWire (compare_faces → CompareDto)', () => {
  it('keeps order, marks the baseline first, and fills every metric key', () => {
    const c = fromWire(dto);
    expect(c.faces).toHaveLength(dto.faces.length);
    expect(c.faces[0].face).toEqual(dto.baseline);
    for (const f of c.faces) expect(Object.keys(f.metrics).sort()).toEqual([...COMPARE_METRICS].sort());
  });

  it('metrics the backend did not send are failed (never zero); sent ones keep value, quality, percentile', () => {
    const c = fromWire(dto);
    const f = c.faces[0]; const a = dto.faces[0].axes.find(x => x.key === 'contrast')!;
    expect(f.metrics.contrast).toMatchObject({ value: a.rawValue, quality: a.quality, percentile: a.percentile });
    expect(f.metrics.x_height_em).toMatchObject({ value: null, quality: 'failed' });
  });

  it('carries the backend deltas (absolute and percent) and leaves the baseline without one', () => {
    const c = fromWire(dto);
    const second = dto.faces[1].axes.find(x => x.key === 'contrast');
    expect(c.faces[0].metrics.contrast.delta).toBeNull();
    if (second) expect(c.faces[1].metrics.contrast.delta).toEqual({ abs: second.delta, pct: second.deltaPct });
  });

  it('estimated flag wins even if quality text differs', () => {
    const copy = JSON.parse(JSON.stringify(dto)) as WireDto;
    copy.faces[0].axes[0] = { ...copy.faces[0].axes[0], quality: 'measured', estimated: true };
    expect(fromWire(copy).faces[0].metrics[copy.faces[0].axes[0].key].quality).toBe('estimated');
  });

  it('exposes Vietnamese overflow as percentages of em, plus extras, family and license note', () => {
    const f = fromWire(dto).faces[0]; const v = dto.faces[0].vietnamese;
    expect(f.vi).toMatchObject({ supports: v.supports, missing: v.missing, stackedMarksOverflow: v.overflow, overAscenderPctEm: v.overAscenderPctEm, overCapPctEm: v.overCapPctEm });
    expect(f.extras?.vendorId).toBe(dto.faces[0].extras.vendorId);
    expect(f.family?.weights).toEqual(dto.faces[0].family.weights);
    expect(f.license).toBe(dto.faces[0].license.kind);
    expect(f.licenseNote?.url).toBe(dto.faces[0].license.url);
  });

  it('tolerates a sparse payload (older analysis: empty extras, missing axes)', () => {
    const copy = JSON.parse(JSON.stringify(dto)) as WireDto;
    (copy.faces[0] as { axes: unknown }).axes = undefined;
    (copy.faces[0] as { extras: unknown }).extras = undefined;
    const f = fromWire(copy).faces[0];
    expect(f.metrics.contrast.quality).toBe('failed');
    expect(f.extras).toBeNull();
  });
});
