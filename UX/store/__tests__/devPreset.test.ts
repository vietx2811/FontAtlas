import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../api/mock';
import { applyDevPreset } from '../devPreset';
import { useUiStore } from '../ui';

const initial = useUiStore.getState();
describe('applyDevPreset (dev-only state from the URL hash)', () => {
  beforeEach(() => useUiStore.setState(initial, true));

  it('selects a family/subfamily, style, tab and expands the info card', async () => {
    await applyDevPreset(createMockApi(), '#sel=Public%20Sans:Regular&style=Bold&tab=vi&info=1&size=50&q=sans');
    const s = useUiStore.getState();
    expect(s.selectedId).toMatch(/:0$/);
    expect([s.style, s.panelTab, s.infoExpanded, s.sampleSize, s.query]).toEqual(['Bold', 'vi', true, 50, 'sans']);
  });

  it('classes sets the class filter', async () => {
    await applyDevPreset(createMockApi(), '#classes=serif,sans');
    expect(useUiStore.getState().filters.classes).toEqual(['serif', 'sans']);
  });
  it('opens Settings', async () => {
    await applyDevPreset(createMockApi(), '#settings=1');
    expect(useUiStore.getState().settingsOpen).toBe(true);
  });

  it('opens a sidebar view', async () => {
    await applyDevPreset(createMockApi(), '#view=star');
    expect(useUiStore.getState().view).toBe('star');
  });

  it('does nothing for an empty or unknown hash', async () => {
    await applyDevPreset(createMockApi(), '');
    await applyDevPreset(createMockApi(), '#sel=Nope');
    expect(useUiStore.getState().selectedId).toBeNull();
  });
});

import { devInitial } from '../devPreset';
describe('devInitial (mock model/embed state from the hash)', () => {
  it('maps model and embed keywords', () => {
    expect(devInitial('#model=downloading').model).toMatchObject({ state: 'downloading' });
    expect(devInitial('#model=checksum').model).toMatchObject({ state: 'checksum_mismatch' });
    expect(devInitial('#embed=running').embed).toMatchObject({ running: true, paused: false, device: 'metal' });
    expect(devInitial('#embed=running').model).toMatchObject({ state: 'ready' });
    expect(devInitial('#embed=paused').embed).toMatchObject({ running: true, paused: true });
    expect(devInitial('').model).toBeUndefined();
  });
});

import { applyDevBoot } from '../devPreset';
describe('applyDevBoot (dev-only data setup from the hash, works on the real backend too)', () => {
  it('adds a source and marks onboarding done with the license acknowledged', async () => {
    const api = createMockApi({ firstRun: true });
    await applyDevBoot(api, '#addsource=/Users/me/FixtureFonts&ob=done');
    expect((await api.listSources()).map(s => s.path)).toContain('/Users/me/FixtureFonts');
    const ob = await api.onboardingState();
    expect([ob.step, ob.licenseNoticeAck]).toEqual(['done', true]);
  });
  it('a nested or duplicate path is ignored quietly; no params does nothing', async () => {
    const api = createMockApi();
    await applyDevBoot(api, '#addsource=/Users/me/Fonts/src1/sub');
    await applyDevBoot(api, '');
    expect((await api.listSources())).toHaveLength(3);
  });
});
