//! Tìm bằng mô tả: văn bản → bộ lọc trục hiện được (không LLM).
use fontatlas_describe::{HardConstraint, Parsed, Strength};

use crate::dto::analysis::{DescribeConflictDto, DescribeDto, MatchedTermDto};
use crate::dto::library::{AxisFilterDto, StrengthDto};
use crate::state::AppState;

pub fn describe(state: &AppState, text: &str) -> DescribeDto {
    from_parsed(state.dictionary.parse(text))
}

pub fn hard_of(names: &[String]) -> Vec<HardConstraint> {
    names
        .iter()
        .filter_map(|n| match n.as_str() {
            "vi_supported" => Some(HardConstraint::ViSupported),
            "vi_complete" => Some(HardConstraint::ViComplete),
            _ => None,
        })
        .collect()
}

fn from_parsed(p: Parsed) -> DescribeDto {
    DescribeDto {
        dictionary_version: p.dictionary_version,
        matched: p
            .matched
            .into_iter()
            .map(|m| MatchedTermDto {
                id: m.id,
                text: m.text,
                group: m.group,
                subjectivity: m.subjectivity,
                culture: m.culture,
                rationale_vi: m.rationale_vi,
                rationale_en: m.rationale_en,
                explanation_banned: m.explanation_banned,
            })
            .collect(),
        filters: p
            .filters
            .into_iter()
            .map(|f| AxisFilterDto {
                axis: f.axis,
                range: f.range.map(|(a, b)| [a, b]),
                equals: f.equals,
                strength: match f.strength {
                    Strength::Required => StrengthDto::Required,
                    Strength::Preferred => StrengthDto::Preferred,
                },
                weight: f.weight,
            })
            .collect(),
        hard: p
            .hard
            .into_iter()
            .map(|h| match h {
                HardConstraint::ViSupported => "vi_supported".to_string(),
                HardConstraint::ViComplete => "vi_complete".to_string(),
            })
            .collect(),
        conflicts: p
            .conflicts
            .into_iter()
            .map(|c| DescribeConflictDto {
                axis: c.axis,
                term_a: c.term_a,
                term_b: c.term_b,
                range_a: c.range_a.map(|(a, b)| [a, b]),
                range_b: c.range_b.map(|(a, b)| [a, b]),
            })
            .collect(),
        unmappable: p.unmappable,
        partial: p.partial,
        unknown_words: p.unknown_words,
    }
}
