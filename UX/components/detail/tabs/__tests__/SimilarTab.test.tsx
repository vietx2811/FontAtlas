import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../../api/mock';
import type { FaceDetailDto } from '../../../../api';
import { faceKey, useUiStore } from '../../../../store/ui';
import { SOURCE_BADGE, SimilarTab } from '../SimilarTab';

const initial = useUiStore.getState();
async function detail(family = 'Roboto'): Promise<FaceDetailDto> {
  const api = createMockApi();
  const r = (await api.queryFaces({ text: family, limit: 1 })).items[0];
  return api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
}

describe('SimilarTab', () => {
  beforeEach(() => useUiStore.setState(initial, true));

  it('shows the 3 nearest fonts, each with "Giống ở … · Khác ở …" and a source badge, and no score', async () => {
    const d = await detail();
    const { container } = render(<SimilarTab detail={d} />);
    const items = screen.getAllByTestId('similar-hit');
    expect(items).toHaveLength(3);
    for (const it of items) {
      expect(it.textContent).toMatch(/Giống ở: .+/);
      expect(it.textContent).toMatch(/Khác ở: .+/);
      expect(['Số đo', 'Số đo + Hình ảnh']).toContain(within(it).getByTestId('source-badge').textContent);
    }
    expect(container.textContent).not.toMatch(/%|\d\.\d/);
  });

  it('axis keys read as Vietnamese names', async () => {
    render(<SimilarTab detail={await detail()} />);
    expect(screen.getAllByTestId('similar-hit')[0].textContent).not.toMatch(/x_height|avg_width|stem_weight/);
  });

  it('badges follow the source', () => {
    expect(SOURCE_BADGE).toEqual({ axes: 'Số đo', axes_and_image: 'Số đo + Hình ảnh', image: 'Hình ảnh' });
  });

  it('clicking a hit selects that face', async () => {
    const d = await detail();
    render(<SimilarTab detail={d} />);
    const hit = d.similarTop[1];
    act(() => { fireEvent.click(screen.getAllByTestId('similar-hit')[1].querySelector('button')!); });
    expect(useUiStore.getState().selectedId).toBe(faceKey(hit.face));
  });

  it('explains when a font has too few measurements', async () => {
    const d = { ...(await detail()), similarTop: [] };
    render(<SimilarTab detail={d} />);
    expect(screen.getByText(/quá ít số đo/)).toBeTruthy();
  });
});
