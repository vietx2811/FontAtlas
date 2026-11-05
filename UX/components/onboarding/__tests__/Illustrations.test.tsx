import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ILLUSTRATED_STEPS, Illustration } from '../Illustrations';

const EMOJI = /[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}\u{FE0F}]/u;

describe('onboarding illustrations', () => {
  it('has one for every step except the scan (which has its own blocks)', () => {
    expect(ILLUSTRATED_STEPS).toEqual(['o1', 'o2', 'o3', 'o5', 'o6', 'o7', 'end']);
  });

  for (const step of ILLUSTRATED_STEPS) {
    describe(step, () => {
      const html = () => { const { container } = render(<Illustration step={step} />); return container; };

      it('is an inline SVG with an accessible name, drawn with a glyph', () => {
        const c = html();
        const svg = c.querySelector('svg')!;
        expect(svg).toBeTruthy();
        expect(screen.getByRole('img').getAttribute('aria-label')).toMatch(/\S/);
        expect(svg.querySelector('text')!.textContent).toMatch(/^\S{1,3}$/u);
      });

      it('uses colour tokens only: no hex or rgb literals, no bitmap, no emoji', () => {
        const c = html();
        expect(c.innerHTML).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
        expect(c.innerHTML).not.toMatch(/rgba?\(/);
        expect(c.querySelector('image, foreignObject')).toBeNull();
        expect(c.textContent).not.toMatch(EMOJI);
      });

      it('keeps stroke/fill colours on theme variables (so it follows light and dark)', () => {
        const c = html();
        const paints = [...c.querySelectorAll('[stroke],[fill]')].flatMap(e => [e.getAttribute('stroke'), e.getAttribute('fill')]).filter(Boolean) as string[];
        for (const p of paints) expect(p, p).toMatch(/^(none|currentColor|var\(--[a-z-]+\))$/);
      });
    });
  }

  it('the welcome art shows a Vietnamese glyph between an x-height line and a baseline', () => {
    const { container } = render(<Illustration step="o1" />);
    expect(container.querySelector('text')!.textContent).toBe('Ặ');
    expect(container.querySelector('[data-guide="baseline"]')).toBeTruthy();
    expect(container.querySelector('[data-guide="x-height"]')).toBeTruthy();
  });

  it('each step has a different picture', () => {
    const svgs = ILLUSTRATED_STEPS.map(s => render(<Illustration step={s} />).container.innerHTML);
    expect(new Set(svgs).size).toBe(ILLUSTRATED_STEPS.length);
  });
});
