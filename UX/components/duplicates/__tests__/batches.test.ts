import { afterEach, describe, expect, it } from 'vitest';
import type { ResolutionDto } from '../../../api';
import { groupBatches } from '../batches';
import { formatBytes } from '../dupText';
import { useNoticeStore } from '../../../store/notices';
import { undoWithFeedback } from '../undoFlow';
import { createMockApi } from '../../../api/mock';

const r = (id: number, batchId: string, action: 'hide' | 'trash', createdAt: number): ResolutionDto => ({ id, batchId, kind: 'same_file', groupKey: `g${id}`, keeper: { hashHex: 'h', faceIndex: 0 }, keeperPath: '/k', loser: { hashHex: 'h', faceIndex: 0 }, loserPath: `/l${id}`, action, trashPath: null, reason: { code: 'tie', args: {} }, wasActive: false, createdAt, undoneAt: null, linksMoved: 0, family: 'F', style: null, status: 'done', keeperActivated: false, canUndo: true });

describe('groupBatches', () => {
  it('keeps unrelated batches apart', () => { expect(groupBatches([r(3, 'b2', 'hide', 1000), r(2, 'b1', 'hide', 990)]).map(b => b.ids)).toEqual([['b2'], ['b1']]); });
  it('joins the Trash batch and the Hide batch of one action (adjacent, different action, close in time)', () => {
    const b = groupBatches([r(4, 'b2', 'hide', 1010), r(3, 'b1', 'trash', 1000), r(2, 'b1', 'trash', 1000)]);
    expect(b).toHaveLength(1); expect(b[0].ids).toEqual(['b2', 'b1']); expect(b[0].rows.map(x => x.id)).toEqual([4, 3, 2]);
  });
  it('does not join when far apart or when both are the same action', () => {
    expect(groupBatches([r(2, 'b2', 'hide', 1100), r(1, 'b1', 'trash', 1000)])).toHaveLength(2);
    expect(groupBatches([r(2, 'b2', 'trash', 1005), r(1, 'b1', 'trash', 1000)])).toHaveLength(2);
  });
});

describe('locale + undo feedback', () => {
  afterEach(() => { document.documentElement.lang = ''; useNoticeStore.setState({ notices: [] }); });
  it('byte sizes follow the current language', () => { expect(formatBytes(1536)).toBe('1,5 KB'); document.documentElement.lang = 'en'; expect(formatBytes(1536)).toBe('1.5 KB'); });
  it('undo errors of one kind collapse into "N files: reason"; many kinds go to the report', async () => {
    const api = createMockApi();
    const fails = (codes: string[]) => ({ batchId: '', done: [], failed: codes.map((code, i) => ({ path: `/x/${i}.ttf`, code, detail: 'raw os text' })), needsRescan: false });
    api.resolveUndo = async () => fails(['trash_emptied', 'trash_emptied', 'trash_emptied']);
    await undoWithFeedback(api, [1]);
    expect(useNoticeStore.getState().notices.map(n => n.text)).toEqual(['3 tệp: tệp đã bị dọn khỏi Thùng rác']);
    useNoticeStore.setState({ notices: [] });
    let reported = 0; api.resolveUndo = async () => fails(['trash_emptied', 'path_occupied', 'trash_mismatch', 'not_found']);
    await undoWithFeedback(api, [1], () => { reported++; });
    expect(reported).toBe(1); expect(useNoticeStore.getState().notices.filter(n => n.kind === 'error')).toHaveLength(0);
  });
});
