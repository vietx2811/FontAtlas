import { describe, expect, it } from 'vitest';
import fixture from '../../bindings/fixtures/duplicate_groups.json';
import summary from '../../bindings/fixtures/duplicate_summary.json';
import { createMockApi } from '../mock';

describe('mock duplicates', () => {
  it('summary counts match the groups per kind and total', async () => {
    const api = createMockApi();
    const [s, g] = [await api.duplicateSummary(), await api.duplicateGroups()];
    const n = (k: string) => g.filter(x => x.kind === k).length;
    expect(s).toEqual({ sameFile: n('same_file'), versions: n('versions'), formats: n('formats'), total: g.length });
    expect(s.total).toBeGreaterThan(0);
  });

  it('every kind appears; same_file members share a face but differ by path; versions mark at most one newest', async () => {
    const g = await createMockApi().duplicateGroups();
    expect(new Set(g.map(x => x.kind))).toEqual(new Set(['same_file', 'versions', 'formats']));
    const same = g.find(x => x.kind === 'same_file')!;
    expect(new Set(same.faces.map(f => f.path)).size).toBe(same.faces.length);
    for (const x of g.filter(x => x.kind === 'versions')) expect(x.faces.filter(f => f.isNewest).length).toBe(x.newestKnown ? 1 : 0);
  });

  it('includes awkward real-world values: unknown format, null version/revision/modified, newest unknown', async () => {
    const g = await createMockApi().duplicateGroups();
    const faces = g.flatMap(x => x.faces);
    expect(faces.some(f => f.format === 'sfnt')).toBe(true);
    expect(faces.some(f => f.revision === null && f.modified === null && f.version === null)).toBe(true);
    expect(g.some(x => x.kind === 'versions' && !x.newestKnown)).toBe(true);
  });

  it('respects the current filters (a query that matches nothing leaves no groups)', async () => {
    const api = createMockApi();
    expect((await api.duplicateGroups({ text: 'zzzz-no-such-font' }))).toEqual([]);
    expect((await api.duplicateSummary({ text: 'zzzz-no-such-font' })).total).toBe(0);
  });

  it('the backend fixture matches the summary fixture', () => {
    const n = (k: string) => fixture.filter(x => x.kind === k).length;
    expect({ sameFile: n('same_file'), versions: n('versions'), formats: n('formats'), total: fixture.length }).toEqual(summary);
  });
});
