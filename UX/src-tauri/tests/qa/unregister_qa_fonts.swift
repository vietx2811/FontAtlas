// Gỡ đăng ký (mọi scope) các font đang nằm trong thư mục QA cũ (thư mục có qa.js bên cạnh A/B/C/E/G): lượt QA bị ngắt giữa chừng để lại
// đăng ký "persistent" trong CoreText, làm lượt sau thấy font của mình là "đã có sẵn · ứng dụng khác" (conflict-clean left:5, available-*).
// Chỉ đụng file nằm dưới thư mục có qa.js hoặc dưới thư mục thử được truyền vào; không bao giờ động tới font hệ thống / thư mục Fonts.
import CoreText
import Foundation
let fm = FileManager.default
let ds = CTFontCollectionCreateMatchingFontDescriptors(CTFontCollectionCreateFromAvailableFonts(nil)) as! [CTFontDescriptor]
// Thư mục thử bổ sung (tham số dòng lệnh): font đăng ký từ file dưới các thư mục này cũng được gỡ (vd. thư mục QA tạm của CI, thư mục thử của 07).
let extraBases = CommandLine.arguments.dropFirst().filter { !$0.isEmpty }.map { URL(fileURLWithPath: $0).standardizedFileURL.path + "/" }
var seen = Set<String>()
for d in ds {
  guard let u = CTFontDescriptorCopyAttribute(d, kCTFontURLAttribute) as? URL, seen.insert(u.path).inserted else { continue }
  let qaDir = u.deletingLastPathComponent().deletingLastPathComponent()
  let underExtra = extraBases.contains { u.standardizedFileURL.path.hasPrefix($0) }
  guard underExtra || fm.fileExists(atPath: qaDir.appendingPathComponent("qa.js").path) else { continue }
  for scope in [CTFontManagerScope.persistent, .session, .user] { CTFontManagerUnregisterFontsForURL(u as CFURL, scope, nil) }
  print("gỡ đăng ký:", u.path)
}
