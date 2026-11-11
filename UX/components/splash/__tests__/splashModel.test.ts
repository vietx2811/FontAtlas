import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { X_HEIGHT, UPM } from '../../onboarding/glyphOutlines';
import { BUNDLED_POOL, NEEDED_GLYPHS, SANS_PART, SERIF_PART, TICKS, WORD, finalFont, isSafeFontPath, isSplashCandidate, lockTick, pickFonts, serifSizeFor, sizeForXHeight, stepSlots, tickDelays, xRatio, type Slot } from '../splashModel';
import { MONO, SANS, SERIF } from '../wordmarkMetrics';

const row = (o: Partial<Parameters<typeof isSplashCandidate>[0]> = {}) => ({ family: 'Fam', faceIndex: 0, fileSize: 120_000, path: '/Library/Fonts/Fam.otf', missing: false, ...o });
const seeded = (seed = 1) => () => { seed = (seed * 1664525 + 1013904223) % 2 ** 32; return seed / 2 ** 32; };

describe('wordmark metrics (hard-coded measurements)', () => {
  it('Inter x-height matches the independent measurement used by the onboarding blueprint (same font file)', () => {
    expect(xRatio(SANS)).toBeCloseTo(X_HEIGHT / UPM, 6);
  });
  it('are plausible x-height/em ratios', () => {
    for (const f of [SERIF, SANS, MONO]) expect(xRatio(f)).toBeGreaterThan(0.4);
    expect(xRatio(SERIF)).toBeLessThan(xRatio(SANS)); // Newsreader nhỏ mắt hơn Inter → phần serif phải to hơn
  });
  it('"Font" in the serif and "Atlas" in the sans end with EQUAL x-height in px at any size', () => {
    for (const atlas of [48, 96, 132]) {
      const serif = serifSizeFor(atlas);
      expect(serif * xRatio(SERIF)).toBeCloseTo(atlas * xRatio(SANS), 6);
      expect(serif).toBeGreaterThan(atlas);
    }
    expect(sizeForXHeight(50, 0.5)).toBe(100);
  });
  it('the final frame is Font = serif, Atlas = sans, from bundled OFL fonts that actually ship', () => {
    expect(SERIF_PART + SANS_PART).toBe(WORD);
    expect(WORD).toBe('FontAtlas');
    [...WORD].forEach((_, i) => expect(finalFont(i).family).toBe(i < 4 ? 'Newsreader' : 'Inter'));
    for (const f of ['newsreader-latin-wght-normal.woff2', 'OFL-Newsreader.txt', 'inter-latin-wght-normal.woff2', 'OFL-Inter.txt']) expect(existsSync(`public/fonts/${f}`), f).toBe(true);
    const css = readFileSync('index.css', 'utf8');
    expect(css).toMatch(/font-family: "Newsreader"/);
    expect(css).toMatch(/newsreader-latin-wght-normal\.woff2/);
  });
  it('every bundled shuffle font has a ratio and is flagged as not user', () => {
    expect(BUNDLED_POOL.length).toBeGreaterThanOrEqual(4);
    for (const p of BUNDLED_POOL) { expect(p.user).toBe(false); expect(p.ratio).toBeGreaterThan(0.3); }
  });
});

describe('random font candidates (library)', () => {
  it('needs every glyph of the wordmark', () => { for (const c of 'Fontlas') expect(NEEDED_GLYPHS).toContain(c); expect(NEEDED_GLYPHS).toContain('A'); });
  it('accepts a normal local font', () => expect(isSplashCandidate(row())).toBe(true));
  it('drops missing files, no path, TTC faces ≥ 1, empty and oversize files', () => {
    expect(isSplashCandidate(row({ missing: true }))).toBe(false);
    expect(isSplashCandidate(row({ path: null }))).toBe(false);
    expect(isSplashCandidate(row({ faceIndex: 2 }))).toBe(false);
    expect(isSplashCandidate(row({ fileSize: 0 }))).toBe(false);
    expect(isSplashCandidate(row({ fileSize: 50 * 1024 * 1024 }))).toBe(false);
  });
  it('accepts ONLY the system and user font folders; anything else could be a dataless cloud placeholder', () => {
    for (const p of ['/Library/Fonts/Arial.ttf', '/System/Library/Fonts/Supplemental/Arial.ttf', '/System/Library/Fonts/Helvetica.ttc', '/Users/me/Library/Fonts/My.otf'])
      expect(isSafeFontPath(p), p).toBe(true);
    for (const p of ['/Users/me/Library/CloudStorage/OneDrive-Personal/Fonts/A.otf', '/Users/me/Library/Mobile Documents/com~apple~CloudDocs/F/A.ttf', '/Users/me/Desktop/Fonts/A.ttf', '/Users/me/Documents/Fonts/A.otf',
      '/Users/me/Nextcloud/Fonts/A.ttf', '/Volumes/Synology/Fonts/A.ttf', '/Users/me/Dropbox/Fonts/A.ttf', 'C:\\Users\\me\\OneDrive\\Fonts\\A.ttf', '/Library/Fonts/../Users/me/Desktop/A.ttf', '/tmp/Library/Fonts/A.ttf'])
      expect(isSafeFontPath(p), p).toBe(false);
    expect(isSplashCandidate(row({ path: '/Users/me/Library/CloudStorage/OneDrive-X/A.otf' }))).toBe(false);
    expect(isSplashCandidate(row({ path: '/Users/me/Desktop/A.otf' }))).toBe(false);
  });
  it('uses the onCloudOnly flag when present: dataless never; already-downloaded cloud fonts allowed; no flag falls back to the folder allowlist', () => {
    const cloud = '/Users/me/Library/CloudStorage/OneDrive-X/A.otf';
    expect(isSplashCandidate(row({ path: cloud, onCloudOnly: true }))).toBe(false);
    expect(isSplashCandidate(row({ path: '/Library/Fonts/A.otf', onCloudOnly: true }))).toBe(false);
    expect(isSplashCandidate(row({ path: cloud, onCloudOnly: false }))).toBe(true);
    expect(isSplashCandidate(row({ path: '/Users/me/Library/../Secret/A.otf', onCloudOnly: false }))).toBe(false);
    expect(isSplashCandidate(row({ path: cloud }))).toBe(false);
  });
  it('picks at most 14 faces, one per family, only candidates, shuffled but deterministic for a seed', () => {
    const rows = Array.from({ length: 60 }, (_, i) => row({ family: `F${i % 30}`, path: i % 5 === 0 ? '/Users/me/Library/CloudStorage/x/a.otf' : `/Library/Fonts/${i}.otf`, faceIndex: i % 7 === 0 ? 1 : 0, missing: i % 11 === 0 }));
    const a = pickFonts(rows, seeded(3)); const b = pickFonts(rows, seeded(3)); const c = pickFonts(rows, seeded(4));
    expect(a.length).toBeLessThanOrEqual(14);
    expect(new Set(a.map(r => r.family)).size).toBe(a.length);
    expect(a.every(r => isSplashCandidate(r))).toBe(true);
    expect(a.map(r => r.path)).toEqual(b.map(r => r.path));
    expect(a.map(r => r.path)).not.toEqual(c.map(r => r.path));
  });
  it('an empty or all-rejected library gives nothing (the splash then shuffles bundled fonts only)', () => {
    expect(pickFonts([], seeded())).toEqual([]);
    expect(pickFonts([row({ missing: true }), row({ path: null })], seeded())).toEqual([]);
  });
});

describe('shuffle rhythm', () => {
  it('slows down and the whole shuffle lasts about 1.5 s (splash is 2 s in total)', () => {
    const d = tickDelays();
    expect(d).toHaveLength(TICKS);
    for (let i = 1; i < d.length; i++) expect(d[i]).toBeGreaterThanOrEqual(d[i - 1]);
    const total = d.reduce((a, b) => a + b, 0);
    expect(total).toBeGreaterThan(1200); expect(total).toBeLessThan(1800);
  });
  it('characters lock left to right, staggered, all done by the last tick', () => {
    const locks = [...WORD].map((_, i) => lockTick(i));
    for (let i = 1; i < locks.length; i++) expect(locks[i]).toBeGreaterThanOrEqual(locks[i - 1]);
    expect(new Set(locks).size).toBeGreaterThan(3);
    expect(Math.max(...locks)).toBeLessThanOrEqual(TICKS);
  });
  it('before locking a char takes a random pool font (never the same twice in a row); after the last tick the word is Font|Atlas', () => {
    const rand = seeded(9);
    let slots: Slot[] = [];
    let prevFirst = '';
    for (let tick = 1; tick <= TICKS; tick++) {
      slots = stepSlots(slots, tick, BUNDLED_POOL, rand);
      if (tick < lockTick(0)) { const f = slots[0].font; expect(slots[0].locked).toBe(false); expect(`${f.family}${f.weight}`).not.toBe(prevFirst); prevFirst = `${f.family}${f.weight}`; }
    }
    expect(slots.every(s => s.locked)).toBe(true);
    expect(slots.map(s => s.font.family)).toEqual([...WORD].map((_, i) => (i < 4 ? 'Newsreader' : 'Inter')));
  });
});
