import CoreText
import Foundation
// In đường dẫn các font đang đăng ký trong CoreText nằm dưới thư mục truyền vào (chỉ đọc).
let base = CommandLine.arguments.count > 1 ? URL(fileURLWithPath: CommandLine.arguments[1]).standardizedFileURL.path + "/" : "/nonexistent/"
let ds = CTFontCollectionCreateMatchingFontDescriptors(CTFontCollectionCreateFromAvailableFonts(nil)) as! [CTFontDescriptor]
var seen = Set<String>()
for d in ds {
  guard let u = CTFontDescriptorCopyAttribute(d, kCTFontURLAttribute) as? URL else { continue }
  let p = u.standardizedFileURL.path
  if p.hasPrefix(base) && seen.insert(p).inserted { print(p) }
}
