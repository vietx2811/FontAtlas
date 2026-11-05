import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../api/mock';
import { startActivationSync } from '../activationSync';
import { useNoticeStore } from '../notices';
import { useUiStore } from '../ui';

const initial = useUiStore.getState();
const settle = () => new Promise(r => setTimeout(r, 0));
const key = (r: { hashHex: string; faceIndex: number }) => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });

describe('startActivationSync', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('keeps the active map in step with changed events', async () => {
    const api = createMockApi();
    const stop = await startActivationSync(api);
    const r = (await api.queryFaces({ limit: 1 })).items[0];
    await api.activationActivate([key(r)]);
    await settle();
    expect([...useUiStore.getState().active.keys()]).toEqual([r.hashHex]);
    stop();
    await api.activationDeactivate([key(r)]);
    await settle();
    expect(useUiStore.getState().active.size).toBe(1); // đã huỷ đăng ký
  });

  it('tells the user when a project change switched fonts off, and when the OS refused', async () => {
    const api = createMockApi();
    const [a, b] = (await api.queryFaces({ limit: 2 })).items;
    const col = await api.createCollection('Menu', null);
    await api.addToCollection(col.id, [key(a), key(b)]);
    await api.activationActivateCollection(col.id);
    await startActivationSync(api);
    await api.removeFromCollection(col.id, [key(a)]);
    await settle();
    expect(useNoticeStore.getState().notices.map(n => n.text)).toEqual(['Đã tự tắt 1 font vì đã rời khỏi dự án.']);
    (api as unknown as { failNextAutoDeactivate: Set<string> }).failNextAutoDeactivate.add(b.hashHex);
    await api.deleteCollection(col.id);
    await settle();
    const last = useNoticeStore.getState().notices.at(-1)!;
    expect(last.kind).toBe('error');
    expect(last.text).toMatch(/Không gỡ được kích hoạt 1 font/);
    expect(useUiStore.getState().active.has(b.hashHex)).toBe(true); // vẫn đăng ký
  });
});
