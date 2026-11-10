import { describe, expect, it } from 'vitest';
import type { SourceDto } from '../../../api';
import { libraryEmpty, type EmptyCtx } from '../zeroStates';

const src = (o: Partial<SourceDto> = {}): SourceDto => ({ id: 1, kind: 'directory', name: 'Fonts', path: '/F', enabled: true, available: true, cloudProvider: null, faceCount: 10, lastScan: null, scanning: false, ...o });
const ctx = (o: Partial<EmptyCtx> = {}): EmptyCtx => ({ view: 'all', tag: null, query: '', viOnly: false, sources: [src()], scanning: false, access: {}, cloud: {}, ...o });

describe('libraryEmpty', () => {
  it('no sources at all: add-folder call to action, mentioning drag and drop', () => {
    const e = libraryEmpty(ctx({ sources: [] }));
    expect(e.kind).toBe('no-sources');
    expect(e.text).toMatch(/Kéo/);
    expect(e.action).toEqual({ id: 'add-folder', label: 'Thêm thư mục font' });
  });
  it('sources exist but nothing yet: scanning says fonts will appear; otherwise offers a rescan', () => {
    const sc = libraryEmpty(ctx({ sources: [src({ faceCount: 0 })], scanning: true }));
    expect(sc.kind).toBe('scanning');
    expect(sc.action).toBeUndefined();
    expect(libraryEmpty(ctx({ sources: [src({ faceCount: 0 })] }))).toMatchObject({ kind: 'sources-empty', action: { id: 'rescan', label: 'Quét lại' } });
  });
  it('filtered no-match never masks no sources, scanning or permission failure', () => {
    const filters = { filtered: true, query: 'x', viOnly: true };
    expect(libraryEmpty(ctx({ ...filters, sources: [] })).kind).toBe('no-sources');
    expect(libraryEmpty(ctx({ ...filters, scanning: true })).kind).toBe('scanning');
    expect(libraryEmpty(ctx({ ...filters, view: 'src:1', access: { 1: 'timeout' } })).kind).toBe('source-access');
    expect(libraryEmpty(ctx({ ...filters, sources: [src({ faceCount: 0 })] })).kind).toBe('sources-empty');
    expect(libraryEmpty(ctx({ filtered: true, familyName: 'Family A' })).text).toContain('Family A');
  });
  it('a search with no hits: names the text and offers to clear the filters', () => {
    const e = libraryEmpty(ctx({ query: 'phở' }));
    expect(e.kind).toBe('no-match');
    expect(e.text).toContain('“phở”');
    expect(e.action).toEqual({ id: 'clear-filters', label: 'Xoá bộ lọc' });
  });
  it('only the Vietnamese filter on: says so', () => {
    const e = libraryEmpty(ctx({ viOnly: true }));
    expect(e.kind).toBe('no-match-vi');
    expect(e.action).toEqual({ id: 'clear-filters', label: 'Bỏ lọc Ặ vi' });
  });
  it('favorites, recent and active are friendly one-liners with no scary action', () => {
    for (const [view, kind] of [['star', 'favorites-empty'], ['recent', 'recent-empty'], ['active', 'active-empty']] as const) {
      const e = libraryEmpty(ctx({ view }));
      expect(e.kind).toBe(kind);
      expect(e.action).toBeUndefined();
    }
    expect(libraryEmpty(ctx({ view: 'star' })).text).toMatch(/ngôi sao/);
    expect(libraryEmpty(ctx({ view: 'active' })).text).toMatch(/công tắc/);
  });
  it('an empty project sends you back to the library to add fonts', () => {
    expect(libraryEmpty(ctx({ view: 'col:3' }))).toMatchObject({ kind: 'project-empty', action: { id: 'go-all', label: 'Thêm font' } });
  });
  it('an empty tag offers to drop the tag', () => {
    expect(libraryEmpty(ctx({ tag: '4' }))).toMatchObject({ kind: 'tag-empty', action: { id: 'clear-filters', label: 'Bỏ thẻ' } });
  });
  it('an unplugged drive says its fonts are still in the library; a plugged empty source offers a rescan', () => {
    const e = libraryEmpty(ctx({ view: 'src:1', sources: [src({ available: false, faceCount: 0 })] }));
    expect(e.kind).toBe('source-unplugged');
    expect(e.text).toMatch(/chưa kết nối/);
    expect(libraryEmpty(ctx({ view: 'src:1', sources: [src({ faceCount: 0 })] })).kind).toBe('source-empty');
  });
  it('the text is short (one line) and never contains an emoji', () => {
    const kinds: Partial<EmptyCtx>[] = [{ sources: [] }, { query: 'x' }, { viOnly: true }, { view: 'star' }, { view: 'recent' }, { view: 'active' }, { view: 'col:1' }, { tag: '1' }, { view: 'src:1', sources: [src({ available: false })] }];
    for (const k of kinds) {
      const e = libraryEmpty(ctx(k));
      expect(e.text.length, e.kind).toBeLessThanOrEqual(72);
      expect(e.text).not.toMatch(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/u);
      expect(e.glyph).toMatch(/^\S$/u);
    }
  });

  it('a source that needs access: grant + retry (two actions), never "no fonts"', () => {
    const e = libraryEmpty(ctx({ view: 'src:1', sources: [src({ faceCount: 0 })], access: { 1: 'timeout' } }));
    expect(e.kind).toBe('source-access');
    expect(e.text).toMatch(/quyền truy cập/);
    expect(e.action).toEqual({ id: 'grant', label: 'Cấp quyền' });
    expect(e.secondary).toEqual({ id: 'rescan', label: 'Thử lại' });
  });
  it('with no fonts at all and every source blocked, the library page says access is needed too', () => {
    const e = libraryEmpty(ctx({ sources: [src({ faceCount: 0 })], access: { 1: 'denied' } }));
    expect(e.kind).toBe('source-access');
  });

  it('a source whose files are all still in the cloud says so (no auto-download), offering a rescan once they are downloaded', () => {
    const e = libraryEmpty(ctx({ view: 'src:1', sources: [src({ faceCount: 0, lastScan: { status: 'ok', seen: 5, unchanged: 0, notDownloaded: 5, relinked: 0, analyzed: 0, failed: 0, missing: 0, unreadable: [] } })] }));
    expect(e.kind).toBe('source-cloud');
    expect(e.text).toMatch(/đám mây/);
    expect(e.action).toEqual({ id: 'rescan', label: 'Quét lại' });
  });
  it('the same for the whole library when every source is only cloud placeholders', () => {
    const stats = { status: 'ok' as const, seen: 3, unchanged: 0, notDownloaded: 3, relinked: 0, analyzed: 0, failed: 0, missing: 0, unreadable: [] };
    expect(libraryEmpty(ctx({ sources: [src({ faceCount: 0, lastScan: stats })] })).kind).toBe('source-cloud');
  });
  it('an empty source with no cloud files stays "source-empty"', () => {
    expect(libraryEmpty(ctx({ view: 'src:1', sources: [src({ faceCount: 0 })] })).kind).toBe('source-empty');
  });
});
