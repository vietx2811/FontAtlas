use fontatlas_store::vi_fold;

#[test]
fn folds_vietnamese_including_d_stroke() {
    assert_eq!(vi_fold("Đẹp Sans"), "dep sans");
    assert_eq!(vi_fold("ĐƯỜNG PHỐ"), "duong pho");
    assert_eq!(vi_fold("Quốc Ngữ"), "quoc ngu");
    assert_eq!(vi_fold("đđĐĐ"), "dddd");
}

#[test]
fn nfc_and_nfd_inputs_fold_the_same() {
    let nfd = "Ho\u{0302}\u{0300}ng"; // "Hồng" dạng NFD
    assert_eq!(vi_fold(nfd), vi_fold("Hồng"));
    assert_eq!(vi_fold(nfd), "hong");
}

#[test]
fn collapses_whitespace_and_trims() {
    assert_eq!(vi_fold("  Phở \t  Bò\n"), "pho bo");
    assert_eq!(vi_fold(""), "");
}

#[test]
fn non_latin_is_kept_lowercased() {
    assert_eq!(vi_fold("東京 Gothic"), "東京 gothic");
    assert_eq!(vi_fold("Ελληνικά"), "ελληνικα");
}
