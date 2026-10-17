//! Index trong bộ nhớ: chuẩn hoá mọi profile một lần, Similar (quét toàn bộ) và ứng viên ghép cặp.
use std::collections::HashMap;

use crate::pair_axes::Thresholds;
use crate::pairing::{FamilyFacts, Gate, PairAssessment, PairContext, assess, rank_key};
use crate::profile::{AxisId, FaceKey, FaceProfile, Q, Role};
use crate::reference::{IndexError, ReferenceTable};
use crate::similar::{
    EMBED_WEIGHT, GroupWeights, MIN_SHARED_DIMS, SimilarHit, SimilarSource, axis_distance,
    embed_cos,
};
use crate::superfamily::SuperfamilyTable;

pub struct Index {
    profiles: Vec<FaceProfile>,
    vectors: Vec<[Option<f32>; 16]>,
    confs: Vec<[f32; 16]>,
    by_key: HashMap<FaceKey, usize>,
    families: HashMap<String, FamilyFacts>,
    reference: ReferenceTable,
    superfamilies: SuperfamilyTable,
    thresholds: Thresholds,
}

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
        profiles: Vec<FaceProfile>,
        reference: ReferenceTable,
        superfamilies: SuperfamilyTable,
    ) -> Self {
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
        Index {
            profiles,
            vectors,
            confs,
            by_key,
            families,
            reference,
            superfamilies,
            thresholds: Thresholds::v1(),
        }
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

    fn distance(&self, i: usize, j: usize, w: &GroupWeights) -> Option<(f32, SimilarSource)> {
        let (d, _) = axis_distance(
            &self.vectors[i],
            &self.confs[i],
            &self.vectors[j],
            &self.confs[j],
            w,
        )?;
        let cos = embed_cos(
            self.profiles[i].embedding.as_deref(),
            self.profiles[j].embedding.as_deref(),
        );
        Some(match cos {
            Some(c) => (
                (1.0 - EMBED_WEIGHT) * d + EMBED_WEIGHT * (1.0 - c) / 2.0,
                SimilarSource::AxesAndImage,
            ),
            None => (d, SimilarSource::Axes),
        })
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
        let mut cands: Vec<(f32, usize, SimilarSource)> = Vec::new();
        for j in 0..self.profiles.len() {
            if j == qi || !filter(&self.profiles[j]) {
                continue;
            }
            if let Some((d, src)) = self.distance(qi, j, w) {
                cands.push((d, j, src));
            }
        }
        let by = |a: &(f32, usize, SimilarSource), b: &(f32, usize, SimilarSource)| {
            a.0.total_cmp(&b.0)
                .then_with(|| self.profiles[a.1].key.cmp(&self.profiles[b.1].key))
        };
        if cands.len() > k && k > 0 {
            cands.select_nth_unstable_by(k - 1, by);
            cands.truncate(k);
        } else {
            cands.truncate(k);
        }
        cands.sort_by(by);
        Ok(cands
            .into_iter()
            .map(|(d, j, source)| {
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
        let ctx = self.ctx(project_vi);
        let mut out: Vec<(usize, PairAssessment)> = Vec::new();
        for (j, pc) in self.profiles.iter().enumerate() {
            if j == qi {
                continue;
            }
            let facts = self.body_facts(pq, q_role, pc, cand_role);
            let x = assess(pq, q_role, pc, cand_role, facts.as_ref(), &ctx);
            if x.gate != Gate::Blocked {
                out.push((j, x));
            }
        }
        let name = |j: usize| format!("{} {}", self.profiles[j].family, self.profiles[j].style);
        let cos = |j: usize| {
            embed_cos(
                pq.embedding.as_deref(),
                self.profiles[j].embedding.as_deref(),
            )
        };
        let mut keyed: Vec<_> = out
            .into_iter()
            .map(|(j, x)| (rank_key(&x, &name(j), cos(j)), j, x))
            .collect();
        keyed.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| self.profiles[a.1].key.cmp(&self.profiles[b.1].key))
        });
        Ok(keyed
            .into_iter()
            .take(k)
            .map(|(_, j, x)| (self.profiles[j].key.clone(), x))
            .collect())
    }
}
