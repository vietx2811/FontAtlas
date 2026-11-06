import { StrictMode } from 'react';
import { act, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { FontFaceLoader, type LoaderDeps } from '../../../features/library-grid/fontFaceLoader';
import { FaceTitle } from '../FaceTitle';

function fakeLoader() {
  const pending: Array<() => void> = [];
  const deps: LoaderDeps = {
    createFace: () => ({ load: () => new Promise(res => pending.push(() => res(null))) }),
    fonts: { add: () => {}, delete: () => true },
    fontUrl: r => (r.faceIndex > 0 ? null : `/font/${r.hashHex}`),
    now: () => 0, velocity: () => 0, setTimer: (fn, ms) => setTimeout(fn, ms), clearTimer: h => clearTimeout(h as never),
  };
  return { loader: new FontFaceLoader(deps), settle: async () => { while (pending.length) pending.shift()!(); await act(async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); }); } };
}
const face = (n: number) => ({ hashHex: n.toString(16).padStart(64, '0'), faceIndex: 0, fileSize: 1000 });

describe('FaceTitle refs', () => {
  it('rapid face changes under StrictMode leave no stray refs after unmount (names are soft refs)', async () => {
    const { loader, settle } = fakeLoader();
    const ui = (n: number) => <StrictMode><FaceTitle loader={loader} face={face(n)}>Tên {n}</FaceTitle></StrictMode>;
    const r = render(ui(1));
    // StrictMode mount → cleanup → mount: acquire/release/acquire phải cân bằng
    expect(loader.stats()).toMatchObject({ refs: 0, soft: 1 });
    for (let n = 2; n <= 8; n++) r.rerender(ui(n)); // đổi face liên tục trước khi kịp nạp
    expect(loader.stats()).toMatchObject({ refs: 0, soft: 1, entries: 1 + 2 }); // tối đa 2 tên đang tải + font hiện tại chờ/đang tải
    await settle();
    r.unmount();
    expect(loader.stats()).toMatchObject({ refs: 0, soft: 0 });
  });

  it('shows the own font once loaded and drops back to the UI font when the face changes', async () => {
    const { loader, settle } = fakeLoader();
    const r = render(<FaceTitle loader={loader} face={face(1)}>A</FaceTitle>);
    await settle();
    const el = () => r.container.firstElementChild as HTMLElement;
    expect(el().style.fontFamily).toContain('fa-');
    expect(el().style.fontFamily).toContain('var(--font-ui)');
    r.rerender(<FaceTitle loader={loader} face={face(2)}>B</FaceTitle>);
    expect(el().style.fontFamily).toBe('');
    await settle();
    expect(el().dataset.ownFont).toContain(face(2).hashHex);
  });
});
