import { describe, expect, it } from 'vitest';
import type { FileOutcomeDto, ReconcileReportDto } from '../../../api';
import { describeOutcome, reconcileNotices } from '../messages';

describe('describeOutcome', () => {
  const t = (o: FileOutcomeDto) => describeOutcome(o);
  it('file_changed asks for a rescan', () => {
    const m = t({ kind: 'failed', code: 'file_changed', message: 'x' });
    expect(m).toMatchObject({ kind: 'error' });
    expect(m!.text).toMatch(/đã thay đổi trên đĩa/);
    expect(m!.text).toMatch(/quét lại/);
  });
  it('other failures show the short code', () => {
    expect(t({ kind: 'failed', code: 'os_denied', message: 'boom' })!.text).toMatch(/os_denied/);
  });
  it('an unsettled row that set_scope could not change is explained, not skipped silently; "already available" says nothing here (the flow toasts once)', () => {
    expect(t({ kind: 'failed', code: 'scope_unsettled', message: 'x' })!.text).toMatch(/tắt rồi bật lại/);
    expect(t({ kind: 'already_available', owner: 'font_base', path: '/x.ttf' })).toBeNull();
  });
  it('missing file and iCloud have their own wording', () => {
    expect(t({ kind: 'missing_file' })!.text).toMatch(/Không tìm thấy file/);
    expect(t({ kind: 'not_downloaded' })!.text).toMatch(/iCloud/);
  });
  it('busy is a retry hint; success outcomes say nothing', () => {
    expect(t({ kind: 'busy' })!.text).toMatch(/thử lại/);
    for (const k of ['activated', 'already_active', 'deactivated', 'already_inactive', 'ready'] as const) expect(t({ kind: k })).toBeNull();
  });
});

describe('reconcileNotices', () => {
  const item = (cause: string, n = 1) => ({ hashHex: 'h', faces: [0], removedReasons: [], cause, count: n });
  const rep = (o: Partial<ReconcileReportDto>): ReconcileReportDto => ({ autoDeactivated: [], autoDeactivateFailed: [], reregistered: 0, moved: 0, missing: 0, notDownloaded: 0, cleaned: 0, failed: 0, results: [], ...o });
  it('names the cause of each automatic deactivation', () => {
    const n = reconcileNotices(rep({ autoDeactivated: [item('collection_deleted'), item('left_collection'), item('no_reason')] }));
    const all = n.map(x => x.text).join('\n');
    expect(all).toMatch(/dự án đã bị xoá/);
    expect(all).toMatch(/rời khỏi dự án/);
    expect(all).toMatch(/không còn lý do kích hoạt/);
    expect(n.every(x => x.kind === 'info')).toBe(true);
  });
  it('reports a refused auto-deactivation as an error: the font is still registered', () => {
    const n = reconcileNotices(rep({ autoDeactivateFailed: [item('left_collection'), item('left_collection')] }));
    expect(n).toHaveLength(1);
    expect(n[0].kind).toBe('error');
    expect(n[0].text).toMatch(/Không gỡ được kích hoạt 2 font/);
    expect(n[0].text).toMatch(/vẫn đang bật/);
  });
  it('summarises the launch reconcile and stays silent when nothing happened', () => {
    expect(reconcileNotices(rep({}))).toEqual([]);
    const n = reconcileNotices(rep({ reregistered: 42, missing: 2 }));
    expect(n[0].text).toBe('Đã kích hoạt lại 42 font. 2 font không tìm thấy file.');
  });
});
