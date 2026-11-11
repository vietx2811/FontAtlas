# "Tắt bản cũ và bật bản này từ Font Atlas" — macOS cho phép gì?

Người viết: 03 · Ngày: 2026-11-11 · Máy đo: macOS 27.2, Apple Silicon · Chưa có code sản phẩm nào đổi.
Ký hiệu: **[ĐO]** = tự chạy trên máy này với font fixture OFL · **[ĐỌC]** = đọc bundle/binary của app đã cài (chỉ đọc) · **[SUY]** = suy luận, chưa kiểm.

## 0. Kết luận ngắn

1. **[ĐO] Một tiến trình khác của cùng người dùng gỡ được font do tiến trình khác đăng ký**, cả scope `session` lẫn `persistent`, miễn đúng **đường dẫn canonical + đúng scope**. Gỡ sai scope trả lỗi 201 vô hại. Scope `process` thì không với tới được. Nghĩa là về kỹ thuật, "Thay thế" làm được cho mọi font **đăng ký qua CoreText API**, bất kể app nào đăng ký.
2. **Không làm được** với font nằm trong thư mục font (`/System/Library/Fonts`, `/Library/Fonts`, `~/Library/Fonts`): hệ điều hành bật chúng theo thư mục, không qua đăng ký API, nên không có gì để gỡ (**[ĐO]** gỡ thứ chưa đăng ký → 201; **[SUY]** áp cho font trong thư mục font, không thử trên thư mục thật theo ràng buộc). Cách duy nhất là xoá/di chuyển file — **cấm**.
3. **Không app nào đã cài có cách tắt từ ngoài**: Typeface chỉ có AppleScript `activatefonts` (+`ping`), không có tắt; RightFont có URL scheme `rightfont://` chỉ với `activate` và `download`, không AppleScript. Tắt font của họ = gỡ đăng ký CoreText phía sau lưng app, app vẫn hiển thị "đang bật".
4. **Đề xuất**: 3 mức — *Thay thế thật* (Font Atlas, app lạ đăng ký qua API), *Thay thế có cảnh báo + hoàn tác* (RightFont, FontBase, Typeface), *Chỉ hướng dẫn* (Adobe, Connect/Suitcase), *Không bao giờ đụng* (3 thư mục font). Chi tiết ở §3.

## 1. Thí nghiệm (font fixture OFL, file copy trong thư mục tạm của phiên làm việc, mỗi bước chạy bằng tiến trình riêng, đã dọn sạch)

Công cụ: một file Swift nhỏ `ct` gọi `CTFontManagerRegisterFontsForURL` / `UnregisterFontsForURL` / `CopyAvailablePostScriptNames` (mã ở Phụ lục). Font dùng: Libre Baskerville, Playfair Display, Source Code Pro, Open Sans (đều OFL, fixture của `fontatlas-core`).

| # | Tình huống | Kết quả |
|---|---|---|
| E1 | A đăng ký `session` và còn sống; tiến trình B gỡ `session` cùng URL | **OK**, font biến mất ngay với mọi tiến trình. B gỡ thêm `persistent` → 201 |
| E2 | A đăng ký `session` rồi thoát; B gỡ `session` | **OK** |
| E3 | A đăng ký `persistent` rồi thoát; B gỡ `session` → **201** (font vẫn còn); B gỡ `persistent` → **OK** | Phải đúng scope |
| E4 | A đăng ký `d1`, B đăng ký `d2` (cùng PostScript name, khác file, cùng `session`) | Cả hai OK, không báo xung đột; gỡ `d1` thì font vẫn còn nhờ `d2` |
| E5 | Gỡ một file chưa từng đăng ký | 201 ở cả `session` và `persistent` |
| E6 | Hai file cùng PostScript name đăng ký lần lượt `d1`, `d2`: tên phân giải về file nào? | **File đăng ký sau thắng** (`d2`) |
| E7 | Chuỗi "thay thế": gỡ bản cũ (thử `session` rồi `persistent`) → đăng ký bản mới | Tên chuyển sang đúng file mới, không còn dấu vết bản cũ |
| E8 | A đăng ký `process`; B xem và gỡ | B không thấy font, gỡ → 201 |

Kết quả khớp bảng đã đo ở `spikes/04_activation.md` §1.1 (hàng 4, 8, 9) và bổ sung điều chưa có ở đó: **gỡ chéo tiến trình** (E1–E3) và **thứ tự thắng khi trùng tên** (E6).

Chưa đo (cần font/app thật của chủ app, ràng buộc không cho phép): font nằm thật trong `~/Library/Fonts` hay `/Library/Fonts`; hành vi của RightFont/Typeface/Adobe khi bị gỡ đăng ký sau lưng.

## 2. Các app quản lý font (chỉ đọc bundle/binary, không bật gì)

| App | Có trên máy | Cơ chế đăng ký | Điều khiển từ ngoài |
|---|---|---|---|
| RightFont 10.2.1 | có | **[ĐỌC]** CoreText tại chỗ, `persistent` (spike 04 §1.4: hàng chục nghìn chuỗi `.rightfontlibrary` trong `fontregistry.user`) | URL scheme `rightfont://` với **`activate`** và `download`; không có `deactivate`; không `NSAppleScriptEnabled`, không sdef. Có tuỳ chọn "Auto-deactivate Fonts" trong app |
| Typeface 4.5.1 | có | chưa xác định scope (không bật thử font nào trong app thật) | AppleScript (`Scriptable.sdef`): **`activatefonts`** (danh sách PostScript name) và `ping`; **không có tắt** |
| FontBase | không | **[SUY]** `session` theo thời gian app chạy (spike 04 §2: "theo phiên app") | Không biết CLI/URL công khai; chưa kiểm |
| Extensis Connect / Suitcase Fusion | không | **[SUY]** có daemon kích hoạt riêng, có thể tự bật lại | Chưa kiểm |
| Adobe Fonts / Creative Cloud | có | File ở `~/Library/Application Support/Adobe/CoreSync/plugins/livetype/` do agent `GudeLivetype` đồng bộ; **[SUY]** agent này đăng ký/gỡ theo trạng thái trên đám mây | Tắt trong Creative Cloud / trang Adobe Fonts; không API công khai |

## 3. Ma trận đề xuất

Cột "Gỡ bản cũ" = Font Atlas có tự làm không. Mọi mức "Thay thế" dùng cùng một chuỗi an toàn (§4). Luôn có "Dùng cả hai" (không đổi gì) như hiện nay.

| Chủ sở hữu (theo `Owner`) | Gỡ được qua CoreText? | Mức đề xuất | Nút / lời hướng dẫn |
|---|---|---|---|
| **Font Atlas** (đường dẫn khác) | Có, [ĐO] E1–E4 | **Thay thế thật** | "Tắt bản cũ và bật bản này" — gỡ bản của chính ta rồi đăng ký bản mới |
| **App lạ / `Other`** đăng ký `session` hoặc `persistent` qua API | Có nếu biết đúng URL + scope, [ĐO] E1–E3 | **Thay thế có xác nhận mạnh + hoàn tác** | Hộp thoại nêu đường dẫn bản cũ, "app đó có thể bật lại khi mở"; có "Khôi phục bản cũ" |
| **RightFont** | Có kỹ thuật ([ĐO] cơ chế `persistent`, E3), nhưng app vẫn ghi "đang bật" | **Thay thế có cảnh báo + hoàn tác** (mặc định chọn "Hướng dẫn") | Hướng dẫn: "Tắt font trong RightFont rồi bấm Thử lại". Nếu chọn thay thế: cảnh báo RightFont sẽ không biết, mở RightFont → Tắt để đồng bộ |
| **Typeface** (hiện rơi vào `Other`) | Như trên; không có lệnh tắt chính thức | **Hướng dẫn** (thay thế như `Other`, có cảnh báo) | "Tắt trong Typeface rồi Thử lại" |
| **FontBase** | **[SUY]** có (session) | **Thay thế có cảnh báo + hoàn tác** | FontBase đăng ký lại khi mở/bật lại; nói rõ |
| **Connect / Suitcase** | **[SUY]** có nhưng daemon bật lại | **Chỉ hướng dẫn** | "Tắt trong Suitcase/Connect rồi Thử lại" |
| **Adobe Fonts / CC** (`…/CoreSync/plugins/livetype/`) | Không nên: agent đồng bộ sẽ bật lại; file do CC quản | **Chỉ hướng dẫn** | "Tắt trong Creative Cloud → Fonts rồi Thử lại". Không đụng file |
| **`/System/Library/Fonts`**, **`/Library/Fonts`** | **Không** (bật theo thư mục) | **Không bao giờ đụng** | Chỉ báo "Font hệ thống, không tắt được". Chọn "Dùng cả hai" (đã có) |
| **`~/Library/Fonts`** (`UserInstalled`) | **Không** (bật theo thư mục) | **Không bao giờ đụng**; **KHÔNG xoá/di chuyển file** | "Hiện trong Finder" + hướng dẫn tự gỡ (vd Font Book → Tắt, **[SUY]** chưa kiểm). Chọn "Dùng cả hai" |

Quy tắc chung: nếu đường dẫn của bản cũ **nằm trong 3 thư mục font ở trên** thì luôn coi là "không bao giờ đụng", bất kể ai đăng ký.

## 4. Chuỗi "Thay thế" an toàn (để code sau này, chưa làm)

1. Lấy **URL của bản đang thắng** từ CoreText (`CTFontDescriptorCopyAttribute(…, kCTFontURLAttribute)`, như E6) và canonicalize. Nếu nằm trong thư mục "không bao giờ đụng" → dừng, chỉ hướng dẫn.
2. Lưu bản ghi hoàn tác: `{path, owner đoán, thời điểm}` (không biết scope → thử cả hai).
3. Gỡ `persistent`, gỡ `session` (một trong hai trả 201 là bình thường, E1/E3). Nếu cả hai đều 201 → bản cũ không phải đăng ký API → báo "không tắt được từ Font Atlas", dừng.
4. Kiểm lại `CopyAvailablePostScriptNames`: tên đã biến mất? Nếu vẫn còn → còn bản khác (E4): lặp với bản kế, tối đa 3 lần rồi dừng.
5. Đăng ký bản của Font Atlas (scope như thiết lập hiện tại), kiểm URL thắng == bản của ta (E6/E7).
6. Ghi `replaced_from` vào `activation_state` để có nút **"Khôi phục bản cũ"**: đăng ký lại đúng đường dẫn cũ với scope `session` (không biết scope gốc thì chọn `session`; đã nêu rõ trong UI).

Rủi ro còn lại:
- App chủ có thể **đăng ký lại** lúc mở hoặc lúc người dùng bật lại → hai bản lại cùng tồn tại, bản đăng ký sau thắng (E6). Không phải lỗi dữ liệu nhưng người dùng cần được báo trước.
- Cửa sổ app đang mở có thể còn dùng font cũ trong bộ nhớ đệm tới khi mở lại tài liệu (đặc điểm chung của CoreText; chưa đo).
- Nếu có hơn một bản cũ cùng tên, chỉ thấy được bản đang thắng; xử lý lặp ở bước 4. `CTFontManagerCopyRegisteredFontDescriptors` (macOS 12+) có thể liệt kê đủ — **[SUY]** chưa thử.

## 5. Câu hỏi cho chủ app

1. Mức mặc định cho RightFont/FontBase/Typeface: "Hướng dẫn" (an toàn) hay "Thay thế có cảnh báo"? Tôi đề xuất **Hướng dẫn là mặc định**, thay thế là lựa chọn thứ hai.
2. Cho phép thử Font Book → Tắt và đo hành vi trên một font cài thủ công vào `~/Library/Fonts` (ngoài ràng buộc hiện tại) để thay dấu **[SUY]** bằng số đo?

## Phụ lục — công cụ đo

```swift
import Foundation; import CoreText
let a = CommandLine.arguments
func scope(_ s: String) -> CTFontManagerScope { s == "persistent" ? .persistent : s == "process" ? .process : .session }
func ok(_ b: Bool, _ e: Unmanaged<CFError>?) -> String { b ? "OK" : "ERR \(e.map { CFErrorGetCode($0.takeRetainedValue()) } ?? -1)" }
switch a[1] {
case "reg":   var e: Unmanaged<CFError>?; print("reg", ok(CTFontManagerRegisterFontsForURL(URL(fileURLWithPath: a[2]) as CFURL, scope(a[3]), &e), e))
              if a.count > 4, let s = Double(a[4]) { fflush(stdout); Thread.sleep(forTimeInterval: s) }
case "unreg": var e: Unmanaged<CFError>?; print("unreg", ok(CTFontManagerUnregisterFontsForURL(URL(fileURLWithPath: a[2]) as CFURL, scope(a[3]), &e), e))
case "has":   print((CTFontManagerCopyAvailablePostScriptNames() as! [String]).contains(a[2]) ? "AVAILABLE" : "absent")
case "url":   let d = CTFontDescriptorCreateWithNameAndSize(a[2] as CFString, 0)
              print((CTFontDescriptorCopyAttribute(d, kCTFontURLAttribute) as? URL)?.path ?? "none")
default: break }
```
