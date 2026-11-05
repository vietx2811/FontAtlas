import { describe, expect, it } from 'vitest';
import { createNoticeStore } from '../notices';

describe('notices', () => {
  it('push assigns ids, dedupes identical live notices, dismisses by id', () => {
    const s = createNoticeStore();
    const a = s.getState().push({ kind: 'info', text: 'x' });
    const b = s.getState().push({ kind: 'info', text: 'x' });
    expect(b).toBe(a);
    s.getState().push({ kind: 'error', text: 'y' });
    expect(s.getState().notices.map(n => n.text)).toEqual(['x', 'y']);
    s.getState().dismiss(a);
    expect(s.getState().notices.map(n => n.text)).toEqual(['y']);
  });
  it('keeps at most 4, dropping the oldest', () => {
    const s = createNoticeStore();
    for (let i = 0; i < 6; i++) s.getState().push({ kind: 'info', text: `n${i}` });
    expect(s.getState().notices.map(n => n.text)).toEqual(['n2', 'n3', 'n4', 'n5']);
  });
});
