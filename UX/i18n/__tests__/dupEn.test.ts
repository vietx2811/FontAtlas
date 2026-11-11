import { describe, expect, it } from 'vitest';
import { DUP_I18N } from '../dup';
import { DUP_EN, DUP_TERMS_EN } from '../en-pending/dup.en';
import { hasKey } from '../index';

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort();
const VI_ONLY = /[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ]/i;
// Dòng mẫu là chữ có dấu tiếng Việt ở cả hai ngôn ngữ (đó là mục đích của nó).
const SAME_IN_BOTH = new Set(['dup.sampleText']);

describe('duplicates strings: English table is complete and consistent', () => {
  it('has exactly the same keys as the Vietnamese table', () => { expect(Object.keys(DUP_EN).sort()).toEqual(Object.keys(DUP_I18N).sort()); });
  it('uses the same {placeholders} in both languages', () => {
    for (const k of Object.keys(DUP_I18N) as Array<keyof typeof DUP_I18N>) expect(placeholders(DUP_EN[k]), k).toEqual(placeholders(DUP_I18N[k]));
  });
  it('has no Vietnamese diacritics in the English text (except the Vietnamese sample line)', () => {
    for (const [k, v] of Object.entries(DUP_EN)) if (!SAME_IN_BOTH.has(k)) expect(v, k).not.toMatch(VI_ONLY);
  });
  it('every code the contract can return has a message (reason, review, warn, fail, blocked)', () => {
    const need = [
      ...['preferred_source', 'same_content_tidiest_path', 'more_complete_vi', 'newer_version', 'preferred_format', 'more_glyphs', 'larger_file', 'chosen_by_user', 'tie'].map(c => `dup.reason.${c}`),
      ...['newest_unknown', 'newer_has_fewer_glyphs', 'preferred_source_is_worse', 'keeper_less_complete_vi', 'variable_vs_static', 'different_license', 'collection_file', 'tie'].map(c => `dup.review.${c}`),
      'dup.warn.may_reflow',
      ...['plan_stale', 'not_auto_safe', 'keeper_missing', 'keeper_not_local', 'keeper_changed', 'keeper_not_in_group', 'keeper_is_hidden', 'keeper_conflict', 'loser_not_in_group', 'not_allowed', 'collection_file', 'active_copy_in_use', 'file_changed', 'deactivate_failed', 'trash_failed', 'not_found', 'already_undone', 'path_occupied', 'trash_emptied', 'trash_mismatch', 'newer_resolution_depends', 'db_error', 'rollback_failed', 'reactivate_failed', 'keeper_activation_failed', 'keeper_deactivate_failed', 'batch_not_found', 'batch_other_session', 'batch_undone'].map(c => `dup.fail.${c}`),
      ...['managed_source', 'system_path', 'outside_source', 'symlink', 'collection_file', 'missing_file'].map(c => `dup.blocked.${c}`),
    ];
    for (const k of need) { expect(hasKey(k), k).toBe(true); expect(k in DUP_EN, k).toBe(true); }
  });
  it('new TermTip terms have gloss in both languages', () => {
    for (const k of Object.keys(DUP_TERMS_EN)) { expect(hasKey(k), k).toBe(true); expect(DUP_TERMS_EN[k as keyof typeof DUP_TERMS_EN]).not.toMatch(VI_ONLY); }
  });
});
