import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Splash } from '../Splash';
import { isSplashOpen, readSplashEnabled, writeSplashEnabled } from '../splashState';
import { createMockApi } from '../../../api/mock';
import { SPLASH_MS } from '../splashModel';

function motion(reduced: boolean) {
  vi.stubGlobal('matchMedia', (q: string) => ({ matches: reduced && q.includes('prefers-reduced-motion'), media: q, addEventListener: () => {}, removeEventListener: () => {} }));
}
beforeEach(() => { vi.useFakeTimers(); motion(false); });
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });
const chars = () => screen.getAllByTestId('splash-char');
const fams = () => chars().map(c => c.getAttribute('data-family'));
const advance = async (ms: number) => { await act(async () => { await vi.advanceTimersByTimeAsync(ms); }); };

describe('Splash', () => {
  it('writes FontAtlas as one word, nine glyphs, no whitespace between the parts', () => {
    const { container } = render(<Splash onDone={() => {}} />);
    expect(chars()).toHaveLength(9);
    expect(container.querySelector('.fa-wm')!.textContent).toBe('FontAtlas');
    expect(container.querySelector('.fa-wm')!.textContent).not.toMatch(/\s/);
    expect(screen.getByRole('status').getAttribute('aria-label')).toBe('Font Atlas');
  });

  it('shuffles fonts while running, then ends on Font = Newsreader (serif) / Atlas = Inter (sans), x-height matched', async () => {
    render(<Splash onDone={() => {}} rand={(() => { let s = 7; return () => (s = (s * 48271) % 2147483647) / 2147483647; })()} />);
    const first = fams().join();
    await advance(300);
    expect(fams().join()).not.toBe(first);
    await advance(1400);
    expect(screen.getByTestId('splash').getAttribute('data-final')).toBe('true');
    expect(fams()).toEqual(['Newsreader', 'Newsreader', 'Newsreader', 'Newsreader', 'Inter', 'Inter', 'Inter', 'Inter', 'Inter']);
    const size = (i: number) => parseFloat(chars()[i].style.fontSize);
    expect(size(0)).toBeGreaterThan(size(4)); // serif cỡ lớn hơn để cùng x-height
    expect(size(4)).toBeCloseTo(1, 6);
    expect(chars().every(c => c.getAttribute('data-locked') === 'true')).toBe(true);
  });

  it('every shuffle font is x-height-normalised relative to the Atlas size', async () => {
    render(<Splash onDone={() => {}} />);
    await advance(200);
    for (const c of chars()) expect(parseFloat(c.style.fontSize)).toBeGreaterThan(0.5);
  });

  it('ends by itself after ~2 s and calls onDone once, after a short fade', async () => {
    const onDone = vi.fn();
    render(<Splash onDone={onDone} />);
    await advance(SPLASH_MS - 50);
    expect(onDone).not.toHaveBeenCalled();
    await advance(400);
    expect(screen.getByTestId('splash').getAttribute('data-leaving')).toBe('true');
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it('any key, or a click, skips it', async () => {
    const a = vi.fn(); const { unmount } = render(<Splash onDone={a} />);
    await advance(100);
    fireEvent.keyDown(document.body, { key: 'x' });
    await advance(300);
    expect(a).toHaveBeenCalledTimes(1);
    unmount();
    const b = vi.fn(); render(<Splash onDone={b} />);
    await advance(100);
    fireEvent.pointerDown(document.body);
    await advance(300);
    expect(b).toHaveBeenCalledTimes(1);
  });

  it('a lone modifier key does not skip it', async () => {
    const onDone = vi.fn(); render(<Splash onDone={onDone} />);
    await advance(100);
    fireEvent.keyDown(document.body, { key: 'Shift' });
    await advance(300);
    expect(onDone).not.toHaveBeenCalled();
  });

  it('prefers-reduced-motion: shows the final frame at once, no shuffling, closes after a short hold', async () => {
    motion(true);
    const onDone = vi.fn(); render(<Splash onDone={onDone} />);
    expect(screen.getByTestId('splash').getAttribute('data-final')).toBe('true');
    expect(fams()).toEqual(['Newsreader', 'Newsreader', 'Newsreader', 'Newsreader', 'Inter', 'Inter', 'Inter', 'Inter', 'Inter']);
    await advance(1300);
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it('hold (QA/screenshots) never ends by itself', async () => {
    const onDone = vi.fn(); render(<Splash onDone={onDone} hold />);
    await advance(10_000);
    expect(onDone).not.toHaveBeenCalled();
  });

  it('does not block or break when the library is empty, or fails: bundled fonts only', async () => {
    const api = { startupStatus: async () => ({ phase: 'ready' }), queryFaces: vi.fn().mockResolvedValue({ total: 0, offset: 0, items: [] }) } as never;
    render(<Splash api={api} onDone={() => {}} />);
    await advance(2000);
    expect(fams()).toHaveLength(9);
    const bad = { startupStatus: async () => ({ phase: 'ready' }), queryFaces: vi.fn().mockRejectedValue(new Error('not ready')) } as never;
    render(<Splash api={bad} onDone={() => {}} />);
    await advance(2000);
    expect(screen.getAllByTestId('splash')).toHaveLength(2);
  });

  it('asks the library only for fonts that cover English fully (every glyph of the word), never loads cloud or missing files', async () => {
    const queryFaces = vi.fn().mockImplementation(async (q: { limit?: number }) => (q.limit === 1
      ? { total: 3, offset: 0, items: [] }
      : { total: 3, offset: 0, items: [
        { hashHex: 'a', faceIndex: 0, family: 'Cloudy', fileSize: 100, path: '/Users/me/Library/CloudStorage/OneDrive/a.otf', missing: false },
        { hashHex: 'b', faceIndex: 0, family: 'Gone', fileSize: 100, path: '/Library/Fonts/b.otf', missing: true },
        { hashHex: 'c', faceIndex: 0, family: 'Good', fileSize: 100, path: '/Library/Fonts/c.otf', missing: false }] }));
    render(<Splash api={{ queryFaces, startupStatus: async () => ({ phase: 'ready' }), fontUrl: () => null } as never} onDone={() => {}} />);
    await advance(100);
    expect(queryFaces).toHaveBeenCalledWith(expect.objectContaining({ langs: [{ code: 'eng', level: 'full' }] }));
  });

  it('does not query the library until the backend is ready (it runs alongside BootGate); bundled fonts only meanwhile', async () => {
    const queryFaces = vi.fn().mockResolvedValue({ total: 0, offset: 0, items: [] });
    render(<Splash api={{ queryFaces, startupStatus: async () => ({ phase: 'scanning' }), fontUrl: () => null } as never} onDone={() => {}} />);
    await advance(2100);
    expect(queryFaces).not.toHaveBeenCalled();
    const ready = vi.fn().mockResolvedValue({ total: 0, offset: 0, items: [] });
    let n = 0;
    render(<Splash api={{ queryFaces: ready, startupStatus: async () => ({ phase: ++n < 3 ? 'indexing' : 'ready' }), fontUrl: () => null } as never} onDone={() => {}} />);
    await advance(1500);
    expect(ready).toHaveBeenCalled();
  });

  it('SWALLOWS the key that skips it: Esc/Enter never reach the guide underneath, and the press is cancelled', async () => {
    const below = vi.fn(); document.addEventListener('keydown', below);
    const onDone = vi.fn(); render(<Splash onDone={onDone} />);
    await advance(100);
    const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    document.body.dispatchEvent(esc);
    const enter = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true });
    document.body.dispatchEvent(enter); // trong 220 ms mờ dần vẫn bị nuốt
    expect(below).not.toHaveBeenCalled();
    expect(esc.defaultPrevented && enter.defaultPrevented).toBe(true);
    await advance(300);
    expect(onDone).toHaveBeenCalledTimes(1);
    document.removeEventListener('keydown', below);
  });

  it('chords (⌘/Ctrl/Alt) are neither a skip nor swallowed; key repeat does not skip but is swallowed', async () => {
    const below = vi.fn(); document.addEventListener('keydown', below);
    const onDone = vi.fn(); render(<Splash onDone={onDone} />);
    await advance(100);
    fireEvent.keyDown(document.body, { key: 'q', metaKey: true });
    expect(below).toHaveBeenCalledTimes(1);
    await advance(200);
    expect(onDone).not.toHaveBeenCalled();
    const rep = new KeyboardEvent('keydown', { key: 'a', repeat: true, bubbles: true, cancelable: true });
    document.body.dispatchEvent(rep);
    expect(rep.defaultPrevented).toBe(true);
    await advance(100);
    expect(screen.getByTestId('splash').getAttribute('data-leaving')).toBe('false');
    document.removeEventListener('keydown', below);
  });

  it('marks itself open while mounted (the guide and shortcuts listen to this)', async () => {
    const { unmount } = render(<Splash onDone={() => {}} />);
    expect(isSplashOpen()).toBe(true);
    unmount();
    expect(isSplashOpen()).toBe(false);
  });
});

describe('splash setting', () => {
  it('is on by default and stored under ui.splash ("off" turns it off) without any backend change', async () => {
    const api = createMockApi();
    expect(await readSplashEnabled(api)).toBe(true);
    await writeSplashEnabled(api, false);
    expect(await api.getSetting('ui.splash')).toBe('off');
    expect(await readSplashEnabled(api)).toBe(false);
    await writeSplashEnabled(api, true);
    expect(await readSplashEnabled(api)).toBe(true);
    expect(await readSplashEnabled({ getSetting: async () => { throw new Error('x'); } })).toBe(true);
  });
});
