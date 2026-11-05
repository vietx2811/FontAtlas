//! Chia nhóm similar_sets thành tune/test: cố định theo seed, phân tầng theo group_type,
//! nhóm chung họ (query ∪ must_include) luôn cùng phía để tune không rò sang test.
use std::collections::BTreeMap;

use crate::csvio::SimilarSet;

/// Đổi seed = đổi phép chia: phải cập nhật cột `split` của CSV và ghi lại lý do.
pub const SIMILAR_SPLIT_SEED: &str = "similar-split-v1";

/// Thành phần trong một tầng: (khoá thứ tự, chỉ số nhóm).
type Comps = Vec<([u8; 32], Vec<usize>)>;

/// id nhóm → "tune" | "test".
/// Thành phần liên thông (qua họ chung, không tính must_not) thuộc tầng group_type của nhóm có id nhỏ nhất.
/// Mỗi tầng n nhóm: duyệt thành phần theo blake3(seed|id nhỏ nhất), đưa vào tune khi tổng ≤ round(n/3)
/// và tầng còn ít nhất một nhóm test.
pub fn split_similar(sets: &[SimilarSet], seed: &str) -> BTreeMap<String, &'static str> {
    let mut parent: Vec<usize> = (0..sets.len()).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut owner: BTreeMap<String, usize> = BTreeMap::new();
    for (i, s) in sets.iter().enumerate() {
        for (fam, _) in std::iter::once(&s.query).chain(&s.must_include) {
            match owner.get(&fam.to_lowercase()) {
                Some(&j) => {
                    let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                    parent[a] = b;
                }
                None => {
                    owner.insert(fam.to_lowercase(), i);
                }
            }
        }
    }
    let mut comps: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..sets.len() {
        let r = root(&mut parent, i);
        comps.entry(r).or_default().push(i);
    }
    // tầng → [(khoá thứ tự, thành phần)]
    let mut strata: BTreeMap<&str, Comps> = BTreeMap::new();
    for members in comps.into_values() {
        let first = *members.iter().min_by_key(|&&i| &sets[i].id).unwrap();
        let key = *blake3::hash(format!("{seed}|{}", sets[first].id).as_bytes()).as_bytes();
        strata
            .entry(&sets[first].group_type)
            .or_default()
            .push((key, members));
    }
    let mut out = BTreeMap::new();
    for mut comps in strata.into_values() {
        comps.sort();
        let n: usize = comps.iter().map(|c| c.1.len()).sum();
        let target = (n + 1) / 3;
        let mut tune = 0;
        for (_, members) in comps {
            let take = tune + members.len() <= target && n > tune + members.len();
            if take {
                tune += members.len();
            }
            for i in members {
                out.insert(sets[i].id.clone(), if take { "tune" } else { "test" });
            }
        }
    }
    out
}
