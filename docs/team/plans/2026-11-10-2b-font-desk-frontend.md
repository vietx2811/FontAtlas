# Luồng 2b · Font Desk (frontend) — plan

Nhánh `feat/font-desk` từ `feat/advanced-search`. Nguồn: `docs/design/font-desk/06_v3_pairing_compare.md` (bố cục C), hợp đồng backend `2026-11-09-1c-case-combo-backend.md` mục "Hợp đồng cho UI". Api + mock Case/Combo đã có (`createCase`… `openCaseFromPeek`, `caseCompare`, `briefStarter`).

## Chia lát (mỗi lát có vitest, QA native, ảnh light/dark; CI chỉ chạy khi Lead cho phép)
1. **2d Pair from peek → Pairing Desk tối thiểu** (lát này): nút "Mở trong Pairing Desk" ở tab Pair của peek (font này là heading/body, ứng viên đang xem thành `trial`); `occupied` → hộp thoại "Thay font X bằng Y?" rồi gọi lại `replace:true`. Màn Desk: cột vai (font viết bằng chính nó), Cuộn chọn cho vai còn lại (nguồn Ghép, `pairCandidates` theo `case.settings`), trang mẫu ghép tiêu đề + đoạn văn, `↑↓` thử, `Enter` đặt (`setComboSlot`), `Esc` về Thư viện.
2. Dải chi tiết (Đang dùng / Đang thử / Δ số đo), nút "Đưa vào Compare".
3. Combo: stepper `Combo n/m`, tạo/nhân bản/đổi tên/đặt chính, lưu `viewState`.
4. Compare (Font rồi Combo): `caseCompare`, mốc cố định, hàng Specimen/Số đo/Meta, Song bản.
5. Brief starter + Case từ Top Bar ("Add to Case").
6. Option Stepper dùng chung, phím `⌘1/⌘2`, nguồn Giống/Cùng họ/Thư viện/Ghim.

## Quyết định cho lát 1
- Trạng thái mở Desk nằm ở `store/ui.ts` (`desk: {caseId, comboId, role, trial} | null`), loại trừ với `compare`/`pairThese`; giống cách `pairThese` thay Thư viện trong `App.tsx`.
- Vai chọn khi mở = `otherRole` (vai còn lại); anchor = font của vai đối diện nhóm (heading-like: display/heading/subheading/accent/quote; body-like: body/caption).
- Chuỗi qua i18n `desk.*`; tên vai là thuật ngữ EN (Display, Body…) có gloss; mã `notes`/lỗi chưa dùng ở lát này.
- `trial` là trạng thái chưa đặt: hiển thị "Đang thử", không ghi DB tới khi `Enter`.
