import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { COMPARE_METRICS } from '../../../api/compare';
import { AXIS_META } from '../../detail/tabs/measurements';
import { TERM_NAMES, TermCard, TermTip, classTermKey, hasTerm, subclassTermKey, termGloss, termKey } from '../terms';

describe('glossary', () => {
  it('every Compare metric and every Anatomy axis has a non-empty gloss', () => {
    for (const k of COMPARE_METRICS) expect(termGloss(k), k).toBeTruthy();
    for (const k of Object.keys(AXIS_META)) expect(hasTerm(k), k).toBe(true);
  });
  it('covers overlay, stacked marks, classification and the classes', () => {
    for (const k of ['overlay', 'vi_marks', 'classification', 'baseline']) expect(termGloss(k), k).toBeTruthy();
    for (const c of ['serif', 'sans', 'mono', 'script', 'blackletter', 'symbol', 'uncertain']) expect(termGloss(classTermKey(c)), c).toBeTruthy();
    expect(termGloss(subclassTermKey('Old style'))).toBeTruthy();
    expect(termGloss(subclassTermKey('neo-grotesque'))).toBeTruthy();
  });
  it('taxonomy v2: every class and every emitted/valid subclass has a name and a gloss; the new terms have an illustration', () => {
    const classes = ['serif', 'slab', 'sans', 'mono', 'script', 'handwriting', 'display', 'blackletter', 'symbol', 'uncertain'];
    const subs = ['venetian_oldstyle', 'garalde', 'transitional', 'didone', 'glyphic', 'wedge', 'clarendon', 'egyptian', 'humanist_slab', 'grotesque', 'neo_grotesque', 'geometric', 'humanist', 'rounded', 'flared', 'squarish', 'sans_mono', 'serif_mono', 'typewriter', 'coding', 'formal_copperplate', 'brush', 'monoline_script', 'upright_script', 'casual_print', 'marker', 'stencil', 'inline_outline', 'pixel_bitmap', 'shaded_3d'];
    for (const c of classes) { expect(termGloss(classTermKey(c)), c).toBeTruthy(); expect(TERM_NAMES[classTermKey(c)], c).toBeTruthy(); }
    for (const x of subs) { expect(termGloss(subclassTermKey(x)), x).toBeTruthy(); expect(TERM_NAMES[subclassTermKey(x)], x).toBeTruthy(); }
    for (const k of ['class.slab', 'class.handwriting', 'class.display', 'sub.flared', 'sub.squarish', 'sub.venetian_oldstyle', 'sub.garalde', 'sub.monoline_script', 'sub.coding']) {
      const { container, unmount } = render(<TermCard term={k} />);
      const art = container.querySelector('svg.fa-term-art'); expect(art, k).toBeTruthy();
      expect(art!.getAttribute('aria-hidden'), k).toBe('true'); // hình minh hoạ chỉ trang trí: chữ giải thích đã đủ nghĩa
      unmount();
    }
  });
  it('maps backend keys to term keys and has an English name for each', () => {
    expect(termKey('x_height_em')).toBe('x_height');
    expect(termKey('slant_deg')).toBe('slant');
    for (const k of COMPARE_METRICS) expect(TERM_NAMES[termKey(k)], k).toBeTruthy();
  });
  it('unknown terms have no gloss (no fake tooltip)', () => {
    expect(termGloss('made_up_axis')).toBeUndefined();
    const { container } = render(<TermTip term="made_up_axis"><b>x</b></TermTip>);
    expect(container.querySelector('b')).toBeTruthy();
  });
});

describe('TermCard / TermTip', () => {
  it('shows name, gloss and a theme-neutral illustration for the listed terms', () => {
    for (const k of ['x_height', 'contrast', 'stress', 'aperture', 'slant', 'serif_foot', 'overlay', 'vi_marks']) {
      const { container, unmount } = render(<TermCard term={k} />);
      expect(container.querySelector('.fa-term b')?.textContent, k).toBe(TERM_NAMES[k]);
      expect(container.querySelector('.fa-term-gloss')?.textContent, k).toBeTruthy();
      const svg = container.querySelector('svg.fa-term-art');
      expect(svg, k).toBeTruthy();
      expect(svg!.outerHTML).not.toMatch(/#[0-9a-f]{3,6}/i); // chỉ currentColor / biến CSS
      unmount();
    }
  });
  it('opens the card on keyboard focus', async () => {
    render(<TermTip term="x_height_em"><b tabIndex={0}>x-height</b></TermTip>);
    await act(async () => { screen.getByText('x-height').focus(); });
    const tip = await screen.findByRole('tooltip');
    expect(tip.textContent).toContain('x-height');
    expect(tip.querySelector('.fa-term-gloss')?.textContent).toMatch(/chữ thường/);
  });
});
