import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ILLUSTRATED_STEPS, Illustration } from '../Illustrations';
import { CAP_HEIGHT, GLYPHS, UPM, X_HEIGHT } from '../glyphOutlines';

const EMOJI = /[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}\u{FE0F}]/u;

describe('onboarding illustrations (blueprint lab)', () => {
  it('has one for every step except the scan (which has its own tray)', () => {
    expect(ILLUSTRATED_STEPS).toEqual(['o1', 'o2', 'o3', 'o5', 'o6', 'o7', 'end']);
  });

  for (const step of ILLUSTRATED_STEPS) {
    describe(step, () => {
      const html = () => render(<Illustration step={step} />).container;

      it('is an inline SVG with an accessible name, drawn with real glyph outlines (paths), not text', () => {
        const c = html();
        expect(screen.getByRole('img').getAttribute('aria-label')).toMatch(/\S{3,}/);
        expect(c.querySelectorAll('path.bp-outline').length).toBeGreaterThan(0);
        for (const p of c.querySelectorAll('g[data-glyph] path')) expect(p.getAttribute('d')).toMatch(/^M-?\d/);
        for (const t of c.querySelectorAll('text')) expect(t.textContent!.length, t.textContent!).toBeLessThanOrEqual(24); // chỉ nhãn kỹ thuật ngắn, không câu chữ (câu chữ đi qua i18n ở HTML)
      });

      it('has no hex/rgb colour, no bitmap, no emoji; paint comes from classes (theme tokens)', () => {
        const c = html();
        expect(c.innerHTML).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
        expect(c.innerHTML).not.toMatch(/rgba?\(/);
        expect(c.querySelector('image, foreignObject')).toBeNull();
        expect(c.textContent).not.toMatch(EMOJI);
        for (const e of c.querySelectorAll('[stroke],[fill]')) {
          for (const v of [e.getAttribute('stroke'), e.getAttribute('fill')].filter(Boolean) as string[]) expect(v, v).toMatch(/^(none|currentColor|url\(#bp-grid-[sl]\)|var\(--[a-z-]+\))$/);
        }
      });

      it('animates only via CSS classes with a delay variable (no inline animation, no JS timers)', () => {
        const c = html();
        expect(c.querySelector('animate, animateTransform, animateMotion, set')).toBeNull();
        expect(c.querySelector('[style*="animation"]')).toBeNull();
      });
    });
  }

  it('welcome: guides sit exactly on the glyph metrics (cap, x-height, baseline) of the real outline', () => {
    const { container } = render(<Illustration step="o1" />);
    const y = (name: string) => Number(container.querySelector(`[data-guide="${name}"]`)!.getAttribute('data-y'));
    const base = y('baseline');
    const size = 210;
    expect(base - y('cap')).toBeCloseTo((CAP_HEIGHT * size) / UPM, 1);
    expect(base - y('x-height')).toBeCloseTo((X_HEIGHT * size) / UPM, 1);
    expect(GLYPHS['Ặ'].top).toBeGreaterThan(CAP_HEIGHT);
    expect(container.querySelector('[data-glyph="Ặ"]')).toBeTruthy();
  });

  it('o1 has short labels beside the guides (term names with a TermTip), and NO long legend block', () => {
    const { container } = render(<Illustration step="o1" />);
    const terms = [...container.querySelectorAll('[data-callout]')].map(e => e.getAttribute('data-callout'));
    expect(terms).toEqual(['cap_height', 'x_height', 'baseline', 'vi_marks']);
    const labels = [...container.querySelectorAll('.bp-label')].map(e => e.textContent);
    expect(labels.every(l => !!l && l.length <= 24)).toBe(true);
    expect(container.querySelector('.fa-legend')).toBeNull();
    expect(container.querySelector('.gloss')).toBeNull();
    for (const l of container.querySelectorAll('.bp-label')) expect(l.getAttribute('tabindex')).toBe('0');
  });

  it('no step shows a legend; activation has its label on the switch', () => {
    for (const s of ILLUSTRATED_STEPS) expect(render(<Illustration step={s} />).container.querySelector('.fa-legend')).toBeNull();
    const { container } = render(<Illustration step="o7" />);
    expect([...container.querySelectorAll('[data-callout]')].map(e => e.getAttribute('data-callout'))).toEqual(['activate']);
  });

  it('each step has a different picture', () => {
    const svgs = ILLUSTRATED_STEPS.map(s => render(<Illustration step={s} />).container.innerHTML);
    expect(new Set(svgs).size).toBe(ILLUSTRATED_STEPS.length);
  });

  it('o4 reserves a slot for the scan tray', () => {
    const got: Array<HTMLElement | null> = [];
    render(<Illustration step="o4" slotRef={el => got.push(el)} />);
    expect(got.some(Boolean)).toBe(true);
  });
});
