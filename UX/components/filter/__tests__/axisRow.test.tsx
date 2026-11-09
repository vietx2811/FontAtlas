import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AxisRow } from '../AxisRow';
import { Histogram } from '../Histogram';
import { RangeSlider } from '../RangeSlider';
import { hasTerm, termKey } from '../../ui/terms';
import { MEASURED_AXES } from '../../../api/mockAnalysis';

describe('every Advanced axis has a TermTip entry', () => {
  it.each([...MEASURED_AXES, 'weight_class', 'width_class'])('%s', axis => { expect(hasTerm(termKey(axis))).toBe(true); });
});

describe('RangeSlider', () => {
  const setup = (over: Partial<React.ComponentProps<typeof RangeSlider>> = {}) => {
    const onChange = vi.fn(); const onReset = vi.fn();
    render(<RangeSlider min={0} max={10} step={1} value={[2, 8]} label="Test" onChange={onChange} onReset={onReset} {...over} />);
    return { onChange, onReset };
  };

  it('keyboard: arrows step, PageUp ×10, Home/End jump, never crossing the other thumb', () => {
    const { onChange } = setup({ min: 0, max: 100 });
    const lo = screen.getByTestId('rs-lo');
    fireEvent.keyDown(lo, { key: 'ArrowRight' }); expect(onChange).toHaveBeenLastCalledWith([3, 8]);
    fireEvent.keyDown(lo, { key: 'PageUp' }); expect(onChange).toHaveBeenLastCalledWith([8, 8]);
    fireEvent.keyDown(lo, { key: 'End' }); expect(onChange).toHaveBeenLastCalledWith([8, 8]);
    fireEvent.keyDown(screen.getByTestId('rs-hi'), { key: 'Home' }); expect(onChange).toHaveBeenLastCalledWith([2, 2]);
  });

  it('discrete ticks: arrows hop between ticks', () => {
    const { onChange } = setup({ ticks: [0, 3, 7, 10], value: [3, 7] });
    fireEvent.keyDown(screen.getByTestId('rs-hi'), { key: 'ArrowRight' });
    expect(onChange).toHaveBeenLastCalledWith([3, 10]);
    fireEvent.keyDown(screen.getByTestId('rs-lo'), { key: 'ArrowLeft' });
    expect(onChange).toHaveBeenLastCalledWith([0, 7]);
  });

  it('double click on the track resets only when a filter is set', () => {
    const a = setup(); fireEvent.doubleClick(screen.getByTestId('rs-track')); expect(a.onReset).toHaveBeenCalledTimes(1);
  });
  it('no reset without a value', () => {
    const a = setup({ value: null }); fireEvent.doubleClick(screen.getByTestId('rs-track')); expect(a.onReset).not.toHaveBeenCalled();
  });

  it('dragging commits once on release, with the snapped value', () => {
    const { onChange } = setup({ value: [0, 10] });
    const track = screen.getByTestId('rs-track');
    track.getBoundingClientRect = () => ({ left: 0, width: 100, top: 0, right: 100, bottom: 4, height: 4, x: 0, y: 0, toJSON() {} });
    fireEvent.pointerDown(screen.getByTestId('rs-hi'), { clientX: 100, pointerId: 1 });
    act(() => { window.dispatchEvent(new MouseEvent('pointermove', { clientX: 52 })); });
    expect(onChange).not.toHaveBeenCalled();
    act(() => { window.dispatchEvent(new MouseEvent('pointerup')); });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith([0, 5]);
  });
});

describe('AxisRow', () => {
  it('shows the English term name with a data-term hook for the TermTip', () => {
    render(<AxisRow axis="x_height_em" min={0} max={1} step={0.01} value={null} onChange={() => {}} />);
    const name = document.querySelector('.fa-axis-name') as HTMLElement;
    expect(name.dataset.term).toBe('x_height'); expect(name.textContent).toBe('x-height');
  });

  it('number inputs commit on Enter; the full range means "no filter"', () => {
    const onChange = vi.fn();
    render(<AxisRow axis="stem" min={0} max={1} step={0.01} value={null} onChange={onChange} />);
    const [lo, hi] = screen.getAllByRole('textbox') as HTMLInputElement[];
    fireEvent.change(lo, { target: { value: '0,2' } }); fireEvent.keyDown(lo, { key: 'Enter' });
    expect(onChange).toHaveBeenLastCalledWith([0.2, 1]);
    fireEvent.change(hi, { target: { value: '5' } }); fireEvent.keyDown(hi, { key: 'Enter' });
    expect(hi.getAttribute('aria-invalid')).toBe('true');
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it('an empty axis renders no slider', () => {
    render(<AxisRow axis="stem" min={0} max={1} step={0.01} value={null} empty onChange={() => {}} />);
    expect(screen.queryByTestId('rs-lo')).toBeNull();
  });
});

describe('Histogram', () => {
  it('draws a dark bar per bin and a light bar only where the span series is larger', () => {
    const { container } = render(<Histogram hist={{ bins: [1, 2, 0], spanBins: [1, 4, 0], empty: false }} />);
    expect(container.querySelectorAll('.fa-hist-bin')).toHaveLength(3);
    expect(container.querySelectorAll('.fa-hist-span')).toHaveLength(1);
  });
  it('draws nothing when the axis has no measurements', () => {
    const { container } = render(<Histogram hist={{ bins: [], spanBins: [], empty: true }} />);
    expect(container.firstChild).toBeNull();
  });
});
