import { describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../mock';
import { withInflightDedupe } from '../inflight';

describe('withInflightDedupe', () => {
  it('lệnh đọc giống hệt đang bay dùng chung một lần gọi; khác tham số thì không', async () => {
    const base = createMockApi();
    const spy = vi.spyOn(base, 'queryFaces');
    const api = withInflightDedupe(base);
    const [a, b, c] = await Promise.all([api.queryFaces({ limit: 1 }), api.queryFaces({ limit: 1 }), api.queryFaces({ limit: 2 })]);
    expect(spy).toHaveBeenCalledTimes(2);
    expect(a).toBe(b); expect(c).not.toBe(a);
    await api.queryFaces({ limit: 1 }); // đã xong → lần gọi mới là lần đọc mới
    expect(spy).toHaveBeenCalledTimes(3);
  });

  it('một lệnh khác (có thể là ghi) xen vào thì bỏ bản đang bay: đọc sau ghi là đọc mới', async () => {
    const base = createMockApi();
    const spy = vi.spyOn(base, 'listSources');
    const api = withInflightDedupe(base);
    const first = api.listSources();
    void api.setFavorite({ hashHex: 'x', faceIndex: 0 }, true).catch(() => undefined);
    const second = api.listSources();
    await Promise.all([first, second]);
    expect(spy).toHaveBeenCalledTimes(2);
  });
});
