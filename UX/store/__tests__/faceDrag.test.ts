// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { beginFaceDrag, cancelFaceDrag, registerFaceDropHandler, targetAt, useFaceDragStore } from '../faceDrag';
import { useSelectionStore } from '../selection';
import { createMockApi } from '../../api/mock';
import { dropFacesInto, dropFacesIntoCollection } from '../../components/dnd/dropFaces';
import { useNoticeStore } from '../notices';
import type { FaceRow } from '../../features/library-grid/types';

const rowsOf = async (api: ReturnType<typeof createMockApi>): Promise<FaceRow[]> => (await api.queryFaces({ limit: 20 })).items as FaceRow[];
const down = (row: FaceRow, x = 10, y = 10, target: EventTarget | null = document.body) => beginFaceDrag({ button: 0, clientX: x, clientY: y, pointerId: 1, target }, row);
const ptr = (type: string, x: number, y: number) => window.dispatchEvent(Object.assign(new Event(type, { bubbles: true }), { clientX: x, clientY: y }));

describe('G1 faceDrag', () => {
  let target: HTMLElement; let at: Element | null;
  beforeEach(() => {
    document.body.innerHTML = '';
    target = document.createElement('div'); target.dataset.dropCol = '7'; target.dataset.dropName = 'Menu'; document.body.appendChild(target);
    at = target;
    (document as unknown as { elementFromPoint: unknown }).elementFromPoint = () => at;
    useSelectionStore.getState().clear(); useNoticeStore.setState({ notices: [] });
  });
  afterEach(() => { cancelFaceDrag(); });

  it('dưới ngưỡng không phải kéo: không có lớp kéo, không gọi handler', async () => {
    const [r] = await rowsOf(createMockApi()); const h = vi.fn(); const off = registerFaceDropHandler(h);
    down(r); ptr('pointermove', 12, 11); ptr('pointerup', 12, 11);
    expect(useFaceDragStore.getState().drag).toBeNull(); expect(h).not.toHaveBeenCalled(); off();
  });

  it('kéo quá ngưỡng rồi thả lên đích: handler nhận đúng font và đích; kéo thẻ ngoài vùng chọn chỉ mang thẻ đó', async () => {
    const [a, b] = await rowsOf(createMockApi()); const h = vi.fn(); const off = registerFaceDropHandler(h);
    useSelectionStore.getState().replace([b], null);
    down(a); ptr('pointermove', 40, 40);
    expect(useFaceDragStore.getState().drag?.rows).toHaveLength(1);
    expect(useFaceDragStore.getState().drag?.over).toMatchObject({ id: 7, valid: true });
    expect(document.body.dataset.faceDragging).toBe('true');
    ptr('pointerup', 40, 40);
    expect(h).toHaveBeenCalledWith([a], expect.objectContaining({ id: 7, name: 'Menu' }));
    expect(useFaceDragStore.getState().drag).toBeNull(); expect(document.body.dataset.faceDragging).toBeUndefined(); off();
  });

  it('kéo thẻ nằm trong vùng chọn mang cả vùng chọn', async () => {
    const [a, b, c] = await rowsOf(createMockApi()); const h = vi.fn(); const off = registerFaceDropHandler(h);
    useSelectionStore.getState().replace([a, b, c], null);
    down(b); ptr('pointermove', 50, 50); expect(useFaceDragStore.getState().drag?.rows).toHaveLength(3); ptr('pointerup', 50, 50);
    expect(h.mock.calls[0][0]).toHaveLength(3); off();
  });

  it('Esc huỷ lượt kéo; thả ở đích "đang xem" (không hợp lệ) hoặc ngoài đích thì không gọi handler', async () => {
    const [a] = await rowsOf(createMockApi()); const h = vi.fn(); const off = registerFaceDropHandler(h);
    down(a); ptr('pointermove', 40, 40);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(useFaceDragStore.getState().drag).toBeNull(); ptr('pointerup', 40, 40); expect(h).not.toHaveBeenCalled();
    target.dataset.dropHere = 'true';
    down(a); ptr('pointermove', 40, 40); expect(useFaceDragStore.getState().drag?.over?.valid).toBe(false); ptr('pointerup', 40, 40);
    expect(h).not.toHaveBeenCalled();
    delete target.dataset.dropHere; at = null;
    down(a); ptr('pointermove', 40, 40); ptr('pointerup', 40, 40); expect(h).not.toHaveBeenCalled(); off();
  });

  it('bấm vào nút con của thẻ (chấm bật, sao) không bắt đầu kéo', async () => {
    const [a] = await rowsOf(createMockApi()); const btn = document.createElement('button'); document.body.appendChild(btn);
    down(a, 10, 10, btn); ptr('pointermove', 80, 80);
    expect(useFaceDragStore.getState().drag).toBeNull();
  });

  it('sau khi thả, click kế tiếp bị nuốt (không đổi vùng chọn)', async () => {
    const [a] = await rowsOf(createMockApi()); const off = registerFaceDropHandler(() => undefined);
    const clicked = vi.fn(); document.body.addEventListener('click', clicked);
    down(a); ptr('pointermove', 40, 40); ptr('pointerup', 40, 40);
    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(clicked).not.toHaveBeenCalled(); off();
  });
});

describe('G1 dropFacesIntoCollection', () => {
  beforeEach(() => useNoticeStore.setState({ notices: [] }));
  it('thêm font chưa có, báo kết quả; Hoàn tác chỉ gỡ phần vừa thêm, không đụng font đã có sẵn', async () => {
    const api = createMockApi(); const [a, b] = await rowsOf(api);
    const col = await api.createCollection('Menu', null);
    await api.addToCollection(col.id, [{ hashHex: a.hashHex, faceIndex: a.faceIndex }]);
    await dropFacesIntoCollection(api, [a, b], col);
    const inCol = async () => (await api.queryFaces({ collectionId: col.id, limit: 50 })).items.map(r => `${r.hashHex}:${r.faceIndex}`);
    expect((await inCol()).sort()).toEqual([`${a.hashHex}:${a.faceIndex}`, `${b.hashHex}:${b.faceIndex}`].sort());
    const n = useNoticeStore.getState().notices.at(-1)!;
    expect(n.text).toMatch(/Đã thêm .* vào «Menu» \(1 font đã có sẵn\)/);
    n.action!.run(); await new Promise(r => setTimeout(r, 0)); await new Promise(r => setTimeout(r, 0));
    expect(await inCol()).toEqual([`${a.hashHex}:${a.faceIndex}`]); // a còn, b đã gỡ
  });
  it('tất cả đã có sẵn: chỉ báo, không có Hoàn tác', async () => {
    const api = createMockApi(); const [a] = await rowsOf(api); const col = await api.createCollection('X', null);
    await api.addToCollection(col.id, [{ hashHex: a.hashHex, faceIndex: a.faceIndex }]);
    await dropFacesIntoCollection(api, [a], col);
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.text).toMatch(/Đã có sẵn trong «X»/); expect(n.action).toBeUndefined();
  });
  it('lỗi từ backend thành thông báo lỗi', async () => {
    const api = createMockApi(); const [a] = await rowsOf(api);
    vi.spyOn(api, 'addToCollection').mockRejectedValue(new Error('boom'));
    await dropFacesIntoCollection(api, [a], { id: 999, name: 'Z' });
    expect(useNoticeStore.getState().notices.at(-1)).toMatchObject({ kind: 'error', text: expect.stringMatching(/Không thêm được vào «Z»: boom/) });
  });
});

describe('G1 thả vào Case', () => {
  beforeEach(() => useNoticeStore.setState({ notices: [] }));
  it('targetAt nhận data-drop-case', () => {
    document.body.innerHTML = '<div data-drop-case="4" data-drop-name="Poster"></div>';
    const el = document.querySelector('div');
    (document as unknown as { elementFromPoint: unknown }).elementFromPoint = () => el;
    expect(targetAt(1, 1)).toEqual({ kind: 'case', id: 4, name: 'Poster', valid: true });
  });
  it('targetAt nhận data-drop-active và data-drop-tag', () => {
    document.body.innerHTML = '<div id="a" data-drop-active="0" data-drop-name="Đang bật"></div><div id="t" data-drop-tag="9" data-drop-name="Logo"></div>';
    const at = (id: string) => { const el = document.getElementById(id); (document as unknown as { elementFromPoint: unknown }).elementFromPoint = () => el; return targetAt(1, 1); };
    expect(at('a')).toEqual({ kind: 'active', id: 0, name: 'Đang bật', valid: true });
    expect(at('t')).toEqual({ kind: 'tag', id: 9, name: 'Logo', valid: true });
  });
  it('thêm vào Case chỉ phần chưa có; Hoàn tác chỉ gỡ phần vừa thêm', async () => {
    const api = createMockApi(); const [a, b] = await rowsOf(api);
    const res = await api.openCaseFromPeek({ face: { hashHex: a.hashHex, faceIndex: a.faceIndex }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    await dropFacesInto(api, [a, b], { kind: 'case', id: res.case.id, name: res.case.name });
    const inCase = async () => (await api.getCase(res.case.id)).faces.map(f => `${f.key.hashHex}:${f.key.faceIndex}`).sort();
    expect(await inCase()).toEqual([`${a.hashHex}:${a.faceIndex}`, `${b.hashHex}:${b.faceIndex}`].sort());
    const n = useNoticeStore.getState().notices.at(-1)!;
    expect(n.text).toMatch(/vào «?Case /);
    n.action!.run(); await new Promise(r => setTimeout(r, 0)); await new Promise(r => setTimeout(r, 0));
    expect(await inCase()).toEqual([`${a.hashHex}:${a.faceIndex}`]);
  });
});
