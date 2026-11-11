import { describe, expect, it } from 'vitest';
import type { PlanItemDto, PlanMemberDto } from '../../../api';
import { memberName, splitForTrash, toRequestItem, trashState, viewOf } from '../dupPlan';
import { blockedText, failText, formatBytes, reasonText, reviewText, warnText } from '../dupText';

const m = (path: string, over: Partial<PlanMemberDto> = {}): PlanMemberDto => ({
  face: { hashHex: 'h', faceIndex: 0 }, family: 'F', style: 'Regular', path, format: 'ttf', version: null, revision: null, modified: null, numGlyphs: 1, fileSize: 1000, viMissing: 0,
  isVariable: false, sourceId: 1, sourceName: 'S', sourceKind: 'directory', active: false, trash: { allowed: true, needsExplicit: false, blocked: null }, ...over,
});
const item = (): PlanItemDto => ({ kind: 'same_file', key: 'k', keeper: m('/a'), losers: [m('/b'), m('/c', { trash: { allowed: true, needsExplicit: true, blocked: null } }), m('/d', { trash: { allowed: false, needsExplicit: false, blocked: 'system_path' } })],
  reason: { code: 'tie', args: {} }, autoSafe: true, review: null, warnings: [], memberPaths: ['/a', '/b', '/c', '/d'] });

describe('viewOf / toRequestItem', () => {
  it('keeps the machine plan when nothing was chosen or the path is unknown', () => {
    const i = item();
    for (const c of [undefined, null, '', '/a', '/zzz']) { const v = viewOf(i, c); expect(v.keeper.path).toBe('/a'); expect(v.chosen).toBe(false); expect(v.reason.code).toBe('tie'); }
  });
  it('swapping makes the old keeper a loser, keeps every member, and says the user chose it', () => {
    const v = viewOf(item(), '/c');
    expect(v.keeper.path).toBe('/c'); expect(v.chosen).toBe(true); expect(v.reason.code).toBe('chosen_by_user');
    expect(v.losers.map(l => l.path).sort()).toEqual(['/a', '/b', '/d']);
    expect(new Set(v.all.map(x => x.path))).toEqual(new Set(['/a', '/b', '/c', '/d']));
  });
  it('the request carries the plan\'s memberPaths verbatim; onlyLosers filters; handled paths leave memberPaths', () => {
    const i = item(); const v = viewOf(i, '/c');
    expect(toRequestItem(i, v)).toEqual({ kind: 'same_file', key: 'k', keeperPath: '/c', loserPaths: ['/a', '/b', '/d'], memberPaths: ['/a', '/b', '/c', '/d'] });
    expect(toRequestItem(i, v, new Set(['/b']), new Set(['/a'])).loserPaths).toEqual(['/b']);
    expect(toRequestItem(i, v, new Set(['/b']), new Set(['/a'])).memberPaths).toEqual(['/b', '/c', '/d']);
  });
});

describe('trash eligibility', () => {
  it('classifies ok / explicit / blocked and splits by the ticks', () => {
    const l = item().losers;
    expect(l.map(trashState)).toEqual(['ok', 'explicit', 'blocked']);
    expect(splitForTrash(l, new Set()).trash.map(x => x.path)).toEqual(['/b']);
    expect(splitForTrash(l, new Set()).hideOnly.map(x => x.path)).toEqual(['/c', '/d']);
    expect(splitForTrash(l, new Set(['/c'])).trash.map(x => x.path)).toEqual(['/b', '/c']);
    expect(splitForTrash(l, new Set(['/d'])).trash.map(x => x.path)).toEqual(['/b']); // ticking a blocked file never helps
  });
  it('names', () => { expect(memberName({ family: 'A', style: null })).toBe('A'); expect(memberName({ family: 'A', style: 'B' })).toBe('A B'); });
});

describe('text from codes', () => {
  it('fills the args and never falls back to a raw code', () => {
    expect(reasonText({ code: 'newer_version', args: { keep: '2.1', other: '2.0' } })).toBe('Bản mới hơn: 2.1 so với 2.0');
    expect(reasonText({ code: 'brand_new_code', args: {} })).toBe('Máy đã chọn bản này');
    expect(reviewText({ code: 'brand_new_code', args: {} })).toBe('Cần bạn xem lại nhóm này');
    expect(warnText({ code: 'x', args: {} })).toBe('Có một lưu ý với nhóm này');
    expect(blockedText('symlink')).toMatch(/liên kết/); expect(blockedText(null)).toBe('không đủ điều kiện'); expect(blockedText('zzz')).toBe('không đủ điều kiện');
    expect(failText({ path: '/x/Foo.ttf', code: 'trash_failed', detail: 'EPERM raw os text' })).toBe('Foo.ttf: không chuyển được vào Thùng rác');
    expect(failText({ path: '/x/Foo.ttf', code: 'zzz', detail: null })).toBe('Foo.ttf: chưa làm được');
  });
  it('formats sizes', () => { expect(formatBytes(0)).toBe('0 B'); expect(formatBytes(1536)).toBe('1,5 KB'); expect(formatBytes(5 * 1024 * 1024)).toBe('5 MB'); expect(formatBytes(-1)).toBe('—'); });
});
