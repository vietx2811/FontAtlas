import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useUiStore } from '../../../store/ui';
import { SampleComposer } from '../SampleComposer';

const initial = useUiStore.getState();
describe('SampleComposer', () => {
  beforeEach(() => useUiStore.setState(initial, true));

  it('starts from the default sample and size 34', () => {
    render(<SampleComposer />);
    expect((screen.getByRole('textbox', { name: 'Chữ mẫu' }) as HTMLInputElement).value).toBe('Phở bò Hà Nội');
    expect((screen.getByRole('slider', { name: 'Cỡ chữ mẫu' }) as HTMLInputElement).value).toBe('34');
  });

  it('typing updates the store at once (no debounce) so grid and panel follow in the same frame', () => {
    render(<SampleComposer />);
    fireEvent.change(screen.getByRole('textbox', { name: 'Chữ mẫu' }), { target: { value: 'Bún chả' } });
    expect(useUiStore.getState().sample).toBe('Bún chả');
  });

  it('presets replace the sample', () => {
    render(<SampleComposer />);
    fireEvent.click(screen.getByRole('button', { name: 'Dấu chồng' }));
    expect(useUiStore.getState().sample).toBe('Ặẫỡự');
    fireEvent.click(screen.getByRole('button', { name: 'Số' }));
    expect(useUiStore.getState().sample).toBe('0123456789');
    fireEvent.click(screen.getByRole('button', { name: 'Aa' }));
    expect(useUiStore.getState().sample).toBe('Aa');
  });

  it('slider spans 18–72 and writes the size', () => {
    render(<SampleComposer />);
    const r = screen.getByRole('slider', { name: 'Cỡ chữ mẫu' }) as HTMLInputElement;
    expect([r.min, r.max]).toEqual(['18', '72']);
    fireEvent.change(r, { target: { value: '50' } });
    expect(useUiStore.getState().sampleSize).toBe(50);
  });

  it('has no formatting controls (G5): only presets and size', () => {
    render(<SampleComposer />);
    expect(screen.getAllByRole('button').map(b => b.getAttribute('aria-label'))).toEqual(['Aa', 'Dấu chồng', 'Số']);
  });
});
