import { describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';

/** "Bật cả Case" ghi lý do `case` (không phải thủ công); tắt Case chỉ gỡ lý do của nó. */
describe('lý do case', () => {
  it('bật cả Case → lý do case; tắt tay một font thì thắng; tắt Case gỡ đúng lý do', async () => {
    const api = createMockApi();
    const c = await api.createCase('Bộ A');
    const faces = (await api.queryFaces({ limit: 2 })).items.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
    await api.addToCase(c.id, faces);
    await api.activationActivateCase(c.id);
    let st = await api.activationState();
    expect(st.length).toBeGreaterThan(0);
    expect(st.every(x => x.reasons.length === 1 && x.reasons[0].kind === 'case' && x.reasons[0].caseId === c.id)).toBe(true);
    // store nạp lý do vào activeReasons
    await useUiStore.getState().refreshActive(api);
    expect([...useUiStore.getState().activeReasons.values()].every(r => r[0].kind === 'case')).toBe(true);
    // font còn lý do bật tay → tắt Case không tắt font
    await api.activationActivate([faces[0]]);
    await api.activationDeactivateCase(c.id);
    st = await api.activationState();
    expect(st.map(x => x.hashHex)).toEqual([faces[0].hashHex]);
    expect(st[0].reasons).toEqual([{ kind: 'manual' }]);
  });
});
