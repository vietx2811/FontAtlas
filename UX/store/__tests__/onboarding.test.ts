import { describe, expect, it } from 'vitest';
import type { OnboardingDto } from '../../api';
import { STEPS, estimateAnalysisMinutes, isFirstRun, needsOnboarding, nextStep, prevStep, startStep, visibleSteps } from '../onboarding';

const base: OnboardingDto = {
  step: null, hasSources: false, legacy: { found: false, preview: null, imported: null, resolved: null, error: null }, licenseNoticeAck: false,
  embedEnabled: false, modelState: 'missing', activationSupported: true, activationOnboarded: false, keepActiveOnExit: false, fontManagers: [],
};
const s = (o: Partial<OnboardingDto>): OnboardingDto => ({ ...base, ...o });
const legacy = { found: true, preview: { fonts: 10, sources: 2, collections: 1, favorites: 3, tags: 2 }, imported: null, resolved: null, error: null };

describe('who sees onboarding', () => {
  it('first run: no step, no sources, license not acknowledged', () => {
    expect(isFirstRun(base)).toBe(true);
    expect(needsOnboarding(base)).toBe(true);
    expect(startStep(base)).toBe('o1');
  });
  it('an unfinished onboarding resumes at the saved step', () => {
    expect(needsOnboarding(s({ step: 'o4' }))).toBe(true);
    expect(startStep(s({ step: 'o4' }))).toBe('o4');
  });
  it('finished or skipped ("done") never comes back by itself', () => {
    expect(needsOnboarding(s({ step: 'done' }))).toBe(false);
  });
  it('anyone who has not acknowledged the license sees onboarding, even with sources already (e.g. imported from the old DB)', () => {
    expect(needsOnboarding(s({ hasSources: true }))).toBe(true);
    expect(isFirstRun(s({ hasSources: true }))).toBe(true);
  });
  it('an acknowledged user with nothing in progress does not', () => {
    expect(needsOnboarding(s({ licenseNoticeAck: true }))).toBe(false);
    expect(needsOnboarding(s({ licenseNoticeAck: true, hasSources: true }))).toBe(false);
  });
  it('skipped (done) without acknowledging: left alone; the license notice comes back at first scan/Similar (spec 2.2 O5), not here', () => {
    expect(needsOnboarding(s({ step: 'done' }))).toBe(false);
  });
  it('an unknown saved step falls back to the first', () => {
    expect(startStep(s({ step: 'zz' }))).toBe('o1');
  });
});

describe('step order', () => {
  it('has the 7 steps then the end screen', () => {
    expect(STEPS).toEqual(['o1', 'o2', 'o3', 'o4', 'o5', 'o6', 'o7', 'end']);
  });
  it('O2 only when legacy data was found and is not yet imported/resolved', () => {
    expect(visibleSteps(base)).not.toContain('o2');
    expect(visibleSteps(s({ legacy }))).toContain('o2');
    expect(visibleSteps(s({ legacy: { ...legacy, imported: { fonts: 1, sources: 1, collections: 0, recoveredCollections: 0, badTags: 0, skippedSources: 0 } } }))).not.toContain('o2');
  });
  it('sources imported from the old database: can add more folders, license, model and activation stay', () => {
    const imported = { ...legacy, imported: { fonts: 1240, sources: 2, collections: 3, recoveredCollections: 0, badTags: 0, skippedSources: 0 } };
    const v = visibleSteps(s({ legacy: imported, hasSources: true }));
    expect(v).toContain('o3');
    expect(v).toEqual(expect.arrayContaining(['o4', 'o5', 'o6', 'o7', 'end']));
    expect(nextStep('o1', s({ legacy: imported, hasSources: true }))).toBe('o3');
    expect(visibleSteps(s({ hasSources: true }))).toContain('o3'); // có nguồn nhưng không từ import: vẫn cho thêm
  });
  it('O4 needs a source', () => {
    expect(visibleSteps(base)).not.toContain('o4');
    expect(visibleSteps(s({ hasSources: true }))).toContain('o4');
  });
  it('O1 → O3 skips O2 without legacy data; O3 → O5 skips the scan when there is no source', () => {
    expect(nextStep('o1', base)).toBe('o3');
    expect(nextStep('o1', s({ legacy }))).toBe('o2');
    expect(nextStep('o3', base)).toBe('o5');
    expect(nextStep('o3', s({ hasSources: true }))).toBe('o4');
    expect(prevStep('o5', base)).toBe('o3');
    expect(prevStep('o3', base)).toBe('o1');
    expect(prevStep('o1', base)).toBeNull();
    expect(nextStep('end', base)).toBeNull();
  });
  it('the dots count only the visible steps (up to 7)', () => {
    expect(visibleSteps(s({ legacy, hasSources: true }))).toHaveLength(8); // + màn kết thúc
  });
});

describe('estimateAnalysisMinutes', () => {
  it('28 ms per face with Metal, rounded up to whole minutes', () => {
    expect(estimateAnalysisMinutes(3800)).toBe(2);
    expect(estimateAnalysisMinutes(100)).toBe(1);
    expect(estimateAnalysisMinutes(0)).toBe(0);
    expect(estimateAnalysisMinutes(40_000)).toBe(19);
  });
});
