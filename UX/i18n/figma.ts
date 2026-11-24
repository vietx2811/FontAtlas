/**
 * Nhập font từ Figma (B6): hộp thoại "Dán từ Figma". Nội dung do người dùng dán rồi xem trước, không tự làm gì.
 * Tên family/style là dữ liệu của file nên giữ nguyên, không dịch. Backend trả mã trạng thái/lỗi, câu chữ nằm ở đây.
 */
export const FIGMA_I18N = {
  'figma.paste': 'Dán từ Figma', 'figma.paste.tip': 'Nhập danh sách font đã sao chép từ plugin Font Atlas trong Figma',
  'figma.title': 'Nhập font từ Figma',
  'figma.pasteHint': 'Trong Figma, chạy plugin Font Atlas, bấm “Sao chép”, rồi dán vào ô bên dưới.',
  'figma.pasteLabel': 'Nội dung sao chép từ plugin Figma', 'figma.read': 'Đọc', 'figma.cancel': 'Huỷ', 'figma.back': 'Dán lại',
  'figma.resolving': 'Đang khớp font với thư viện…',
  'figma.summary': 'Tìm thấy {n} font: {exact} có trong thư viện, {ambiguous} cần chọn bản, {rest} chưa có.',
  'figma.fromFile': 'Từ file Figma “{file}”', 'figma.omitFile': 'Không dùng tên file Figma',
  'figma.st.exact': 'Có trong thư viện', 'figma.st.ambiguous': 'Có nhiều bản, chọn một',
  'figma.st.family_only': 'Có họ này nhưng không có style {style}', 'figma.st.none': 'Không có trong thư viện',
  'figma.figmaMissing': 'Figma báo font này đang thiếu trên máy bạn', 'figma.fileMissing': 'Font có trong thư viện nhưng file không còn trên đĩa',
  'figma.pick': 'Chọn một bản cho {family} {style}', 'figma.pick.none': 'Chưa chọn', 'figma.include': 'Đưa {family} {style} vào Case',
  'figma.create': 'Tạo Case ({n} font)', 'figma.create.none': 'Chọn ít nhất một font để tạo Case',
  'figma.caseName': 'Figma: {file}', 'figma.caseNameNoFile': 'Từ Figma',
  'figma.done': 'Đã tạo Case “{name}” với {n} font.', 'figma.fail': 'Không tạo được Case: {msg}',
  'figma.linkError': 'Link Figma không dùng được: {msg}',
} as const;

export const FIGMA_EN = {
  'figma.paste': 'Paste from Figma', 'figma.paste.tip': 'Import the font list you copied from the Font Atlas plugin in Figma',
  'figma.title': 'Import fonts from Figma',
  'figma.pasteHint': 'In Figma, run the Font Atlas plugin, press “Copy”, then paste below.',
  'figma.pasteLabel': 'Text copied from the Figma plugin', 'figma.read': 'Read', 'figma.cancel': 'Cancel', 'figma.back': 'Paste again',
  'figma.resolving': 'Matching fonts with your library…',
  'figma.summary': 'Found {n} fonts: {exact} in your library, {ambiguous} need a choice, {rest} not found.',
  'figma.fromFile': 'From Figma file “{file}”', 'figma.omitFile': 'Don’t use the Figma file name',
  'figma.st.exact': 'In your library', 'figma.st.ambiguous': 'Several copies, pick one',
  'figma.st.family_only': 'You have this family, but not the {style} style', 'figma.st.none': 'Not in your library',
  'figma.figmaMissing': 'Figma reports this font as missing on your machine', 'figma.fileMissing': 'The font is in your library but its file is no longer on disk',
  'figma.pick': 'Pick a copy of {family} {style}', 'figma.pick.none': 'Not chosen', 'figma.include': 'Add {family} {style} to the Case',
  'figma.create': 'Create Case ({n} fonts)', 'figma.create.none': 'Pick at least one font to create a Case',
  'figma.caseName': 'Figma: {file}', 'figma.caseNameNoFile': 'From Figma',
  'figma.done': 'Created Case “{name}” with {n} fonts.', 'figma.fail': 'Could not create the Case: {msg}',
  'figma.linkError': 'That Figma link can’t be used: {msg}',
} as const;
