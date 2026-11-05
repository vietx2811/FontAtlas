import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../api/mock';
import { useNoticeStore } from '../notices';
import { createScanStore, startScanSync } from '../scan';
import { PRIVACY_URLS, openPrivacySettings, setUrlOpener } from '../../privacy';

const denied = (reason: 'permission_denied' | 'timeout') => ({ code: 'io', message: 'x', detail: { source: 'app', op: 'opendir', reason, path: '/Volumes/X/Fonts' } });
beforeEach(() => useNoticeStore.setState({ notices: [] }));

async function setup() {
  const api = createMockApi();
  let push: (e: import('../../api').ScanEvent) => void = () => {};
  vi.spyOn(api, 'scanSubscribe').mockImplementation(async l => { push = l; return () => {}; });
  const s = createScanStore();
  await startScanSync(api, s);
  return { api, s, push: (e: import('../../api').ScanEvent) => push(e) };
}

describe('source access problems', () => {
  it('a refused or timed-out open marks the source as needing access and tells the user, with a way to grant it', async () => {
    const { s, push } = await setup();
    push({ type: 'queued', sourceId: 3 });
    push({ type: 'failed', sourceId: 3, error: denied('timeout') });
    await new Promise(r => setTimeout(r, 0));
    expect(s.getState().access[3]).toBe('timeout');
    expect(s.getState().scanning).toBe(false);
    const n = useNoticeStore.getState().notices;
    expect(n).toHaveLength(1);
    expect(n[0].kind).toBe('error');
    expect(n[0].text).toMatch(/cần quyền truy cập/);
    expect(n[0].text).toContain('Ổ ngoài'); // tên nguồn id 3 trong mock
    expect(n[0].action?.label).toBe('Cấp quyền');
  });

  it('permission_denied is distinguished from timeout', async () => {
    const { s, push } = await setup();
    push({ type: 'failed', sourceId: 2, error: denied('permission_denied') });
    expect(s.getState().access[2]).toBe('denied');
  });

  it('an ordinary scan error is not an access problem', async () => {
    const { s, push } = await setup();
    push({ type: 'failed', sourceId: 2, error: { code: 'io', message: 'hỏng', detail: { source: 'store' } } });
    expect(s.getState().access[2]).toBeUndefined();
  });

  it('retrying (queued) and finishing clear the flag; fonts already scanned are untouched', async () => {
    const { api, s, push } = await setup();
    push({ type: 'failed', sourceId: 3, error: denied('timeout') });
    expect(s.getState().access[3]).toBe('timeout');
    push({ type: 'queued', sourceId: 3 });
    expect(s.getState().access[3]).toBeUndefined();
    push({ type: 'failed', sourceId: 3, error: denied('timeout') });
    push({ type: 'finished', sourceId: 3, stats: { status: 'ok', seen: 1, unchanged: 1, relinked: 0, analyzed: 0, failed: 0, missing: 0, unreadable: [] } });
    expect(s.getState().access[3]).toBeUndefined();
    expect((await api.queryFaces({ sourceIds: [3] })).total).toBeGreaterThan(0); // vẫn còn font của nguồn đó
  });
});

describe('openPrivacySettings', () => {
  it('opens Privacy & Security › Files and Folders by default, Full Disk Access on request', async () => {
    const opened: string[] = [];
    setUrlOpener(async u => { opened.push(u); return true; });
    expect(await openPrivacySettings()).toBe(true);
    await openPrivacySettings('disk');
    expect(opened).toEqual([PRIVACY_URLS.files, PRIVACY_URLS.disk]);
    expect(PRIVACY_URLS.files).toMatch(/^x-apple\.systempreferences:.*Privacy_FilesAndFolders$/);
    expect(PRIVACY_URLS.disk).toMatch(/Privacy_AllFiles$/);
  });
  it('reports failure when the opener throws, so the UI can show the manual path instead', async () => {
    setUrlOpener(async () => { throw new Error('no'); });
    expect(await openPrivacySettings()).toBe(false);
  });
});
