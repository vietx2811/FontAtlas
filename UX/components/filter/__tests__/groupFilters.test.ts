import { describe, expect, it } from 'vitest';
import { groupFilters } from '../groupFilters';

describe('groupFilters', () => {
  it('is empty without filters', () => { expect(groupFilters({})).toEqual([]); });

  it('orders groups in a fixed order regardless of key order', () => {
    const g = groupFilters({ rawAxisRanges: [{ axis: 'x_height_ratio', range: [0.4, 0.6] }], license: ['ofl'], italic: true, classes: ['serif'], vi: 'full' });
    expect(g.map(x => x.id)).toEqual(['class', 'style', 'vi', 'license', 'axes']);
  });

  it('puts every value of a group in its chips', () => {
    const g = groupFilters({ classes: ['serif', 'sans'], subclasses: ['didone'] });
    expect(g).toHaveLength(1);
    expect(g[0].chips).toHaveLength(3);
  });

  it('removing a group drops only its keys', () => {
    const f = { classes: ['serif'], license: ['ofl'], italic: true };
    const cls = groupFilters(f).find(x => x.id === 'class')!;
    expect(cls.remove(f)).toEqual({ license: ['ofl'], italic: true });
  });

  it('removing one chip of a group keeps the rest of that group', () => {
    const f = { classes: ['serif', 'sans'] };
    const g = groupFilters(f)[0];
    expect(g.chips[0].remove(f)).toEqual({ classes: ['sans'] });
  });

  it('ignores favorites when the view already forces it', () => {
    expect(groupFilters({ favoritesOnly: true }, [], 'star')).toEqual([]);
    expect(groupFilters({ favoritesOnly: true }, [], 'all').map(x => x.id)).toEqual(['library']);
  });
});

describe('tag filter (filters.tagIds)', () => {
  const tags = [{ id: 4, name: 'serif' }, { id: 7, name: 'retro' }];
  it('shows one chip per tag in its own group, named from the tags list; removing one keeps the others, the last drops the key', () => {
    const g = groupFilters({ tagIds: [4, 7] }, [], 'all', tags).find(x => x.id === 'tags')!;
    expect(g.label).toBe('Thẻ');
    expect(g.chips.map(c => c.label)).toEqual(['Thẻ: serif', 'Thẻ: retro']);
    expect(g.chips[0].remove({ tagIds: [4, 7] })).toEqual({ tagIds: [7] });
    expect(g.chips[1].remove({ tagIds: [7] })).toEqual({});
    expect(g.remove({ tagIds: [4, 7] })).toEqual({});
  });
  it('an unknown tag id still shows a chip so it can be removed', () => {
    expect(groupFilters({ tagIds: [99] }).find(x => x.id === 'tags')!.chips[0].label).toBe('Thẻ: #99');
  });
});
