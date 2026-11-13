// Viết tay (không do ts-rs sinh): Rust `pub type ComboRoleDto = String` (src-tauri/src/dto/cases.rs), nên ts-rs không xuất file này.
// Vai trong Combo là chuỗi tuỳ ý (0.4.0 G3), 1–40 ký tự, không phân biệt hoa thường trong cùng Combo.
// 7 khoá gợi sẵn: 'display' | 'heading' | 'subheading' | 'body' | 'caption' | 'accent' | 'quote'.
export type ComboRoleDto = string;
