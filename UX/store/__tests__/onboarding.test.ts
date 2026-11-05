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
  it('an existing user (data present, nothing saved) is not treated as a first run', () => {
    expect(needsOnboarding(s({ hasSources: true }))).toBe(false);
    expect(needsOnboarding(s({ licenseNoticeAck: true }))).toBe(false);
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
