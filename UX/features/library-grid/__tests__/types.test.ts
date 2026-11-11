import { describe, expect, it } from 'vitest';
// Fixture do store sinh từ serde thật (test face_row_json_matches_contract_fixture của Plan 2).
import storeRow from '../../../src-tauri/crates/fontatlas-store/tests/fixtures/face_row.json';
import { ContractError, cssSize, parseFacePage, parseFaceRow } from '../types';

// App thêm `langBadge` (Hyperglot) lên 20 khoá của store.
const row = { ...storeRow, langBadge: null };

describe('hợp đồng FaceRow với store', () => {
  it('accepts the Rust-generated fixture', () => {
    const r = parseFaceRow(row);
    expect(r.fileSize).toBe(12345);
    expect(r.hashHex).toHaveLength(64);
    expect(parseFacePage({ total: 1, offset: 0, items: [row] }).items).toHaveLength(1);
  });
  it('accepts null where Rust has Option', () => {
    expect(() => parseFaceRow({ ...row, subfamily: null, fullName: null, weightClass: null, widthClass: null, path: null })).not.toThrow();
  });
  it('rejects missing, mistyped or unexpected fields with their path', () => {
    const { fileSize: _, ...missing } = row;
    expect(() => parseFacePage({ total: 1, offset: 0, items: [missing] })).toThrow(ContractError);
    expect(() => parseFacePage({ total: 1, offset: 0, items: [missing] })).toThrow(/items\[0\]\.fileSize/);
    expect(() => parseFaceRow({ ...row, faceIndex: '0' })).toThrow(/faceIndex/);
    expect(() => parseFaceRow({ ...row, family: null })).toThrow(/family/);
    expect(() => parseFaceRow({ ...row, newField: 1 })).toThrow(/unexpected key newField/);
  });
});

describe('cssSize', () => {
  it('is half of the Rust canvas', () => {
    expect(cssSize(48)).toEqual({ width: 312, height: 56 });
    expect(cssSize(32)).toEqual({ width: 208, height: 37 });
  });
});
