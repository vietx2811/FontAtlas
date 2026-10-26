//! Index trong bộ nhớ: chuẩn hoá mọi profile một lần, Similar (quét toàn bộ) và ứng viên ghép cặp.
use std::collections::HashMap;

use rayon::prelude::*;

use crate::embed_index::{ExactEmbeddings, Q8, RERANK_CANDIDATES};
use crate::pair_axes::Thresholds;
use crate::pairing::{
    FamilyFacts, Gate, PairAssessment, PairContext, RankParts, assess, rank_parts,
};
use crate::profile::{AxisId, FaceKey, FaceProfile, Q, Role};
use crate::reference::{IndexError, ReferenceTable};
use crate::similar::{
    EMBED_WEIGHT, GroupWeights, MAX_EMBED_WEIGHT, MIN_SHARED_DIMS, SimilarHit, SimilarSource,
    axis_distance, combine, embed_cos,
};
use crate::superfamily::SuperfamilyTable;

pub struct Index {
    profiles: Vec<FaceProfile>,
    vectors: Vec<[Option<f32>; 16]>,
    confs: Vec<[f32; 16]>,
    by_key: HashMap<FaceKey, usize>,
    families: HashMap<String, FamilyFacts>,
    /// FamilyFacts của họ từng profile (tính sẵn cho pair_candidates).
    facts: Vec<FamilyFacts>,
    /// "Family Style" từng profile — khoá phụ khi xếp hạng.
    names: Vec<String>,
    reference: ReferenceTable,
    superfamilies: SuperfamilyTable,
    thresholds: Thresholds,
    /// Embedding ảnh (int8); f32 không giữ trong profile.
    q8: Q8,
    exact: Option<Box<dyn ExactEmbeddings>>,
    embed_w: f32,
}

/// Ứng viên Similar: (khoảng cách kết hợp, khoảng cách trục, chỉ số, nguồn).
type Cand = (f32, f32, usize, SimilarSource);

fn in_range(v: f32, lo: f32, hi: f32) -> bool {
    v >= lo && v <= hi
}

/// Face này có đóng góp Regular (≈400) / Bold (≈700) cho họ không.
fn face_weights(p: &FaceProfile) -> (bool, bool) {
    let wc = p.weight_class.map(f32::from);
    let range = |t: f32| p.wght_range.is_some_and(|(lo, hi)| in_range(t, lo, hi));
    let named = |n: &str| p.named_instances.iter().any(|x| x.eq_ignore_ascii_case(n));
    let regular = wc.is_some_and(|w| in_range(w, 350.0, 450.0)) || range(400.0) || named("Regular");
    let bold = wc.is_some_and(|w| in_range(w, 650.0, 750.0)) || range(700.0) || named("Bold");
    (regular, bold)
}

impl Index {
    pub fn new(
        mut profiles: Vec<FaceProfile>,
        reference: ReferenceTable,
        superfamilies: SuperfamilyTable,
    ) -> Self {
        // Một đường duy nhất: embedding f32 của profile → int8 trong Index, rồi bỏ f32 (RAM ~dim B/face).
        let embs: Vec<Option<Vec<f32>>> = profiles.iter_mut().map(|p| p.embedding.take()).collect();
        let q8 = Q8::build(&embs);
        drop(embs);
        let vectors = profiles.iter().map(|p| reference.normalize(p)).collect();
        let confs = profiles
            .iter()
            .map(|p| {
                let mut c = [0.0f32; 16];
                for (i, a) in AxisId::ALL.iter().enumerate() {
                    c[i] = p.get(*a).q.conf();
                }
                c
            })
            .collect();
        let by_key = profiles
            .iter()
            .enumerate()
            .map(|(i, p)| (p.key.clone(), i))
            .collect();
        let mut families: HashMap<String, FamilyFacts> = HashMap::new();
        for p in &profiles {
            let (r, b) = face_weights(p);
            let f = families.entry(p.family.clone()).or_insert(FamilyFacts {
                has_regular: false,
                has_bold: false,
            });
            f.has_regular |= r;
            f.has_bold |= b;
        }
        let facts = profiles
            .iter()
            .map(|p| {
                families.get(&p.family).copied().unwrap_or(FamilyFacts {
                    has_regular: false,
                    has_bold: false,
                })
            })
            .collect();
        let names = profiles
            .iter()
            .map(|p| format!("{} {}", p.family, p.style))
            .collect();
        Index {
            facts,
            names,
            profiles,
            vectors,
            confs,
            by_key,
            families,
            reference,
            superfamilies,
            thresholds: Thresholds::v1(),
            q8,
            exact: None,
            embed_w: EMBED_WEIGHT,
        }
    }

    /// Nguồn f32 cho bước xếp lại `RERANK_CANDIDATES` ứng viên đầu. Không có → dùng cos int8.
    pub fn set_exact_source(&mut self, src: Box<dyn ExactEmbeddings>) {
        self.exact = Some(src);
    }

    /// Trọng số ảnh, kẹp vào [0, MAX_EMBED_WEIGHT]; không hữu hạn → mặc định.
    pub fn set_embed_weight(&mut self, w: f32) {
        self.embed_w = if w.is_finite() {
            w.clamp(0.0, MAX_EMBED_WEIGHT)
        } else {
            EMBED_WEIGHT
        };
    }

    pub fn embed_weight(&self) -> f32 {
        self.embed_w
    }

    /// Face có embedding ảnh dùng được trong Index.
    pub fn has_image(&self, k: &FaceKey) -> bool {
        self.by_key.get(k).is_some_and(|&i| self.q8.has(i))
    }

    pub fn profile(&self, k: &FaceKey) -> Option<&FaceProfile> {
        self.by_key.get(k).map(|&i| &self.profiles[i])
    }

    #[doc(hidden)]
    pub fn profiles_for_test(&self) -> &[FaceProfile] {
        &self.profiles
    }

    pub fn reference(&self) -> &ReferenceTable {
        &self.reference
    }

    pub fn family_facts(&self, family: &str) -> FamilyFacts {
        self.families.get(family).copied().unwrap_or(FamilyFacts {
            has_regular: false,
            has_bold: false,
        })
    }

    fn idx(&self, k: &FaceKey) -> Result<usize, IndexError> {
        self.by_key
            .get(k)
            .copied()
            .ok_or_else(|| IndexError::NotFound(k.clone()))
    }

    fn distance(&self, i: usize, j: usize, w: &GroupWeights) -> Option<Cand> {
        let (d, _) = axis_distance(
            &self.vectors[i],
            &self.confs[i],
            &self.vectors[j],
            &self.confs[j],
            w,
        )?;
        Some(match self.q8.cos(i, j) {
            Some(c) => (
                combine(d, c, self.embed_w),
                d,
                j,
                SimilarSource::AxesAndImage,
            ),
            None => (d, d, j, SimilarSource::Axes),
        })
    }

    /// Xếp lại bằng f32: đọc vector của query + ứng viên có ảnh, tính lại khoảng cách kết hợp.
    fn rerank_exact(&self, qi: usize, cands: &mut [Cand]) {
        let Some(src) = self.exact.as_ref() else {
            return;
        };
        if !self.q8.has(qi) {
            return;
        }
        let idx: Vec<usize> = cands
            .iter()
            .enumerate()
            .filter(|(_, c)| c.3 == SimilarSource::AxesAndImage)
            .map(|(n, _)| n)
            .collect();
        let mut keys = vec![self.profiles[qi].key.clone()];
        keys.extend(idx.iter().map(|&n| self.profiles[cands[n].2].key.clone()));
        let got = src.fetch(&keys);
        let Some(Some(qv)) = got.first() else { return };
        for (&n, v) in idx.iter().zip(got.iter().skip(1)) {
            if let Some(c) = embed_cos(Some(qv), v.as_deref()) {
                cands[n].0 = combine(cands[n].1, c, self.embed_w);
            }
        }
    }

    /// "Giống ở" / "Khác ở" trên các trục cả hai đo thật (Measured).
    fn explain(&self, i: usize, j: usize) -> (Vec<AxisId>, Vec<AxisId>) {
        let mut d: Vec<(f32, AxisId)> = AxisId::ALL
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                self.profiles[i].get(**a).q == Q::Measured
                    && self.profiles[j].get(**a).q == Q::Measured
            })
            .filter_map(|(k, a)| Some(((self.vectors[i][k]? - self.vectors[j][k]?).abs(), *a)))
            .filter(|(x, _)| x.is_finite())
            .collect();
        d.sort_by(|x, y| x.0.total_cmp(&y.0));
        let same = d
            .iter()
            .filter(|x| x.0 <= 0.10)
            .take(3)
            .map(|x| x.1)
            .collect();
        let differ = d
            .iter()
            .rev()
            .filter(|x| x.0 >= 0.25)
            .take(3)
            .map(|x| x.1)
            .collect();
        (same, differ)
    }

    pub fn similar(
        &self,
        q: &FaceKey,
        w: &GroupWeights,
        k: usize,
        filter: &dyn Fn(&FaceProfile) -> bool,
    ) -> Result<Vec<SimilarHit>, IndexError> {
        let qi = self.idx(q)?;
        if self.vectors[qi].iter().filter(|v| v.is_some()).count() < MIN_SHARED_DIMS {
            return Err(IndexError::NotComparable(q.clone()));
        }
        let mut cands: Vec<Cand> = (0..self.profiles.len())
            .filter(|&j| j != qi && filter(&self.profiles[j]))
            .filter_map(|j| self.distance(qi, j, w))
            .collect();
        let by = |a: &Cand, b: &Cand| {
            a.0.total_cmp(&b.0)
                .then_with(|| self.profiles[a.2].key.cmp(&self.profiles[b.2].key))
        };
        // Lượt 1 (int8): giữ max(k, RERANK_CANDIDATES) ứng viên; lượt 2: xếp lại f32 nếu có nguồn.
        let m = k.max(RERANK_CANDIDATES);
        if cands.len() > m {
            cands.select_nth_unstable_by(m - 1, by);
            cands.truncate(m);
        }
        self.rerank_exact(qi, &mut cands);
        cands.sort_by(by);
        cands.truncate(k);
        Ok(cands
            .into_iter()
            .map(|(d, _, j, source)| {
                let (same, differ) = self.explain(qi, j);
                SimilarHit {
                    key: self.profiles[j].key.clone(),
                    distance: d,
                    source,
                    same,
                    differ,
                }
            })
            .collect())
    }

    fn ctx(&self, project_vi: bool) -> PairContext<'_> {
        PairContext {
            thresholds: &self.thresholds,
            reference: &self.reference,
            superfamilies: &self.superfamilies,
            project_vi,
        }
    }

    fn body_facts(
        &self,
        a: &FaceProfile,
        ra: Role,
        b: &FaceProfile,
        rb: Role,
    ) -> Option<FamilyFacts> {
        if rb == Role::Body {
            Some(self.family_facts(&b.family))
        } else if ra == Role::Body {
            Some(self.family_facts(&a.family))
        } else {
            None
        }
    }

    pub fn assess_pair(
        &self,
        a: &FaceKey,
        ra: Role,
        b: &FaceKey,
        rb: Role,
        project_vi: bool,
    ) -> Result<PairAssessment, IndexError> {
        let (pa, pb) = (&self.profiles[self.idx(a)?], &self.profiles[self.idx(b)?]);
        let facts = self.body_facts(pa, ra, pb, rb);
        Ok(assess(
            pa,
            ra,
            pb,
            rb,
            facts.as_ref(),
            &self.ctx(project_vi),
        ))
    }

    pub fn pair_candidates(
        &self,
        q: &FaceKey,
        q_role: Role,
        cand_role: Role,
        project_vi: bool,
        k: usize,
    ) -> Result<Vec<(FaceKey, PairAssessment)>, IndexError> {
        let qi = self.idx(q)?;
        let pq = &self.profiles[qi];
        // H1 lọc trước: face truy vấn thiếu tiếng Việt → mọi cặp Blocked; ứng viên thiếu → Blocked.
        if project_vi && pq.vi_missing > 0 {
            return Ok(Vec::new());
        }
        let ctx = self.ctx(project_vi);
        // Cos ảnh (int8) chỉ là tie-break ở bước 5 của rank_key; không ảnh hưởng cổng.
        let cos = |j: usize| self.q8.cos(qi, j);
        // FamilyFacts của bên Body (giống body_facts, nhưng tra mảng tính sẵn).
        let facts = |j: usize| {
            if cand_role == Role::Body {
                Some(self.facts[j])
            } else if q_role == Role::Body {
                Some(self.facts[qi])
            } else {
                None
            }
        };
        // Song song theo ứng viên; collect giữ thứ tự nên kết quả xác định.
        let mut keyed: Vec<(RankParts, usize, PairAssessment)> = self
            .profiles
            .par_iter()
            .enumerate()
            .filter(|(j, pc)| *j != qi && !(project_vi && pc.vi_missing > 0))
            .filter_map(|(j, pc)| {
                let x = assess(pq, q_role, pc, cand_role, facts(j).as_ref(), &ctx);
                (x.gate != Gate::Blocked).then(|| (rank_parts(&x, cos(j)), j, x))
            })
            .collect();
        // Cùng thứ tự với rank_key (phần số, rồi tên), rồi key.
        let by = |a: &(RankParts, usize, PairAssessment),
                  b: &(RankParts, usize, PairAssessment)| {
            a.0.cmp(&b.0)
                .then_with(|| self.names[a.1].cmp(&self.names[b.1]))
                .then_with(|| self.profiles[a.1].key.cmp(&self.profiles[b.1].key))
        };
        // Thứ tự toàn phần → chọn top-k rồi sort riêng phần đó cho ra đúng top-k của sort toàn bộ.
        if k > 0 && keyed.len() > k {
            keyed.select_nth_unstable_by(k - 1, by);
            keyed.truncate(k);
        }
        keyed.sort_by(by);
        Ok(keyed
            .into_iter()
            .take(k)
            .map(|(_, j, x)| (self.profiles[j].key.clone(), x))
            .collect())
    }
}
