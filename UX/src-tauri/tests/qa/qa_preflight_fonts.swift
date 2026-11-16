// Preflight QA: font fixture của lượt QA này CHƯA được đăng ký ở nơi khác trên máy (cài đặt, Adobe Fonts, app khác, lượt QA cũ).
// Nếu cùng PostScript name đã đăng ký từ một file KHÁC thì các bước kích hoạt của QA sẽ ra «Chưa bật» / «1 font đã có trên máy».
//   swift qa_preflight_fonts.swift <thư mục fixture> [thư mục fixture …]
// In "FONT-REGISTERED <PostScript> <đường dẫn>" cho từng font trùng; thoát 2 nếu có, 0 nếu sạch. Chỉ đọc, không đăng ký/gỡ gì.
import CoreText
import Foundation
let fm = FileManager.default
let dirs = Array(CommandLine.arguments.dropFirst())
var ps = [String: String]()   // PostScript name -> file fixture
for d in dirs {
  guard let items = try? fm.contentsOfDirectory(atPath: d) else { continue }
  for f in items.sorted() where ["ttf", "otf", "ttc"].contains((f as NSString).pathExtension.lowercased()) {
    let url = URL(fileURLWithPath: d).appendingPathComponent(f)
    guard let arr = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor] else { continue }
    for desc in arr { if let n = CTFontDescriptorCopyAttribute(desc, kCTFontNameAttribute) as? String { ps[n] = url.path } }
  }
}
let own = dirs.map { URL(fileURLWithPath: $0).standardizedFileURL.path + "/" }
let all = CTFontCollectionCreateMatchingFontDescriptors(CTFontCollectionCreateFromAvailableFonts(nil)) as! [CTFontDescriptor]
var found = Set<String>()
for d in all {
  guard let n = CTFontDescriptorCopyAttribute(d, kCTFontNameAttribute) as? String, ps[n] != nil,
        let u = CTFontDescriptorCopyAttribute(d, kCTFontURLAttribute) as? URL else { continue }
  let p = u.standardizedFileURL.path
  if own.contains(where: { p.hasPrefix($0) }) { continue }
  found.insert("FONT-REGISTERED \(n) \(p)")
}
for l in found.sorted() { print(l) }
exit(found.isEmpty ? 0 : 2)
