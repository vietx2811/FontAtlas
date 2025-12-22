/*!
 * Qdrant Integration Module
 * ==========================
 * 
 * Provides vector search capabilities for font similarity and pairing.
 * Integrates with Qdrant vector database for perceptual and metric-based search.
 */

use qdrant_client::{Qdrant, QdrantError};
use qdrant_client::qdrant::{SearchPoints, PointStruct, Value, Filter, Condition, FieldCondition, Match};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const QDRANT_URL: &str = "http://localhost:6334";
const COLLECTION_NAME: &str = "font_profiles";  // Updated to match actual collection
const METRIC_VECTOR_SIZE: usize = 9;

/// Result from a similarity search
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimilarFont {
    pub font_id: String,
    pub name: String,
    pub file_path: String,
    pub similarity_score: f32,
    pub category: Option<String>,
    pub primary_class: Option<String>,
    pub foundry: Option<String>,
    // Metrics for pairing analysis
    pub x_height: f32,
    pub width_variance: f32,
    pub stroke_contrast: f32,
    pub aperture: f32,
}

/// Font pairing result with enhanced scoring
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontPairing {
    pub font_id: String,
    pub font_name: String,
    pub file_path: String,
    pub score: f32,
    pub visual_similarity: f32,
    pub metric_harmony: MetricHarmony,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricHarmony {
    pub x_height: f32,
    pub contrast: f32,
    pub width: f32,
}

// ============================================================================
// Enhanced Pairing Structures (2-3 font combos, multiple logics)
// ============================================================================

/// Individual font entry in a combo
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontComboEntry {
    pub font_id: String,
    pub font_name: String,
    pub file_path: String,
    pub role: String,  // "headline", "body", "accent", "caption"
    pub primary_class: Option<String>,
}

/// A combination of 2-3 fonts that work well together
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontCombo {
    pub fonts: Vec<FontComboEntry>,
    pub score: f32,
    pub rationale: String,
}

/// Pairing results grouped by logic/algorithm
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PairingByLogic {
    pub logic_name: String,   // "Classic Contrast", "Harmonic Unity", "Modern Tech"
    pub logic_key: String,    // "contrast", "harmony", "modern"
    pub description: String,
    pub results: Vec<FontCombo>,
}


/// Qdrant client wrapper
pub struct QdrantService {
    client: Qdrant,
}

impl QdrantService {
    /// Create new Qdrant service  
    pub async fn new() -> Result<Self, String> {
        log::info!("Connecting to Qdrant at {}", QDRANT_URL);
        let client = Qdrant::from_url(QDRANT_URL)
            .build()
            .map_err(|e| {
                let err = format!("Failed to connect to Qdrant ({}): {}", QDRANT_URL, e);
                log::error!("{}", err);
                err
            })?;

        Ok(Self { client })
    }

    /// Find similar fonts using perceptual (CLIP) vector search
    pub async fn find_similar_by_perceptual(
        &self,
        font_id: &str,
        limit: usize,
    ) -> Result<Vec<SimilarFont>, String> {
        // First, get the target font's perceptual vector
        let points = self.client
            .get_points(
                qdrant_client::qdrant::GetPoints {
                    collection_name: COLLECTION_NAME.to_string(),
                    ids: vec![font_id.to_string().into()],
                    with_payload: Some(true.into()),
                    with_vectors: Some(true.into()),
                    ..Default::default()
                }
            )
            .await
            .map_err(|e| format!("Failed to get font for perceptual search: {}", e))?;

        if points.result.is_empty() {
            return Err("Font not found in Qdrant".to_string());
        }

        let target_point = &points.result[0];

        // Extract 'perceptual' vector
        let vector = match self.get_named_vector(target_point, "perceptual") {
            Ok(v) => v,
            Err(_) => {
                log::warn!("Font {} has no perceptual vector, falling back to metrics", font_id);
                return self.find_similar_by_metrics(font_id, limit).await;
            }
        };

        // Search using 'perceptual' vector
        let search_result = self.client
            .search_points(SearchPoints {
                collection_name: COLLECTION_NAME.to_string(),
                vector,
                vector_name: Some("perceptual".to_string()),
                limit: (limit + 1) as u64,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Perceptual search failed: {}", e))?;

        Ok(self.map_search_results(search_result, font_id, limit))
    }

    /// Internal helper to extract a named vector from a point
    fn get_named_vector(&self, point: &qdrant_client::qdrant::RetrievedPoint, name: &str) -> Result<Vec<f32>, String> {
        if let Some(vectors) = &point.vectors {
            if let Some(opts) = &vectors.vectors_options {
                match opts {
                    qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v) => Ok(v.data.clone()),
                    qdrant_client::qdrant::vectors_output::VectorsOptions::Vectors(named) => {
                        named.vectors.get(name)
                            .map(|v| v.data.clone())
                            .ok_or_else(|| format!("No '{}' vector found", name))
                    }
                }
            } else {
                Err("No vector options found".to_string())
            }
        } else {
            Err("Point has no vectors".to_string())
        }
    }

    /// Internal helper to map search results to SimilarFont
    fn map_search_results(&self, search_result: qdrant_client::qdrant::SearchResponse, exclude_id: &str, limit: usize) -> Vec<SimilarFont> {
        search_result.result
            .into_iter()
            .filter(|p| {
                if let Some(id) = &p.id {
                    let id_str = match &id.point_id_options {
                        Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(n)) => n.to_string(),
                        Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u)) => u.clone(),
                        _ => return true,
                    };
                    return id_str != exclude_id;
                }
                true
            })
            .take(limit)
            .filter_map(|point| {
                let payload = point.payload;
                let point_id = match point.id? {
                    qdrant_client::qdrant::PointId { point_id_options: Some(opts) } => {
                        match opts {
                            qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n.to_string(),
                            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => u,
                        }
                    },
                    _ => return None,
                };

                Some(SimilarFont {
                    font_id: point_id,
                    name: get_string_field(&payload, "name")?,
                    file_path: get_string_field(&payload, "path")?,
                    similarity_score: point.score,
                    category: get_string_field(&payload, "category"),
                    primary_class: get_string_field(&payload, "primary_class"),
                    foundry: get_string_field(&payload, "foundry"),
                    x_height: get_float_field(&payload, "x_height").unwrap_or(0.0),
                    width_variance: get_float_field(&payload, "width_variance").unwrap_or(0.0),
                    stroke_contrast: get_float_field(&payload, "stroke_contrast").unwrap_or(0.0),
                    aperture: get_float_field(&payload, "aperture").unwrap_or(0.0),
                })
            })
            .collect()
    }

    /// Get vectors for a specific font
    pub async fn get_font_vectors(&self, font_id: &str) -> Result<std::collections::HashMap<String, Vec<f32>>, String> {
        let points = self.client
            .get_points(
                qdrant_client::qdrant::GetPoints {
                    collection_name: COLLECTION_NAME.to_string(),
                    ids: vec![font_id.to_string().into()],
                    with_vectors: Some(true.into()),
                    ..Default::default()
                }
            )
            .await
            .map_err(|e| format!("Failed to get vectors for font {}: {}", font_id, e))?;

        if points.result.is_empty() {
            return Err("Font not found".to_string());
        }

        let point = &points.result[0];
        let mut result = std::collections::HashMap::new();

        if let Some(vectors) = &point.vectors {
            if let Some(opts) = &vectors.vectors_options {
                match opts {
                    qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v) => {
                        result.insert("default".to_string(), v.data.clone());
                    },
                    qdrant_client::qdrant::vectors_output::VectorsOptions::Vectors(named) => {
                        for (name, vec) in &named.vectors {
                            result.insert(name.clone(), vec.data.clone());
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Calculate cosine similarity between two vectors
    pub fn calculate_cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
        cosine_similarity(v1, v2)
    }

    /// Find similar fonts using metric-based vector search
    pub async fn find_similar_by_metrics(
        &self,
        font_id: &str,
        limit: usize,
    ) -> Result<Vec<SimilarFont>, String> {
        // First, get the target font's metrics vector
        let points = self.client
            .get_points(
                qdrant_client::qdrant::GetPoints {
                    collection_name: COLLECTION_NAME.to_string(),
                    ids: vec![font_id.to_string().into()],
                    with_payload: Some(true.into()),
                    with_vectors: Some(true.into()),
                    ..Default::default()
                }
            )
            .await
            .map_err(|e| format!("Failed to get font for metrics search: {}", e))?;

        if points.result.is_empty() {
            return Err("Font not found in Qdrant".to_string());
        }

        let target_point = &points.result[0];
        let vector = self.get_named_vector(target_point, "metrics")?;

        // Search points
        let search_result = self.client
            .search_points(SearchPoints {
                collection_name: COLLECTION_NAME.to_string(),
                vector,
                vector_name: Some("metrics".to_string()),
                limit: (limit + 1) as u64,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Metrics search failed: {}", e))?;

        Ok(self.map_search_results(search_result, font_id, limit))
    }

    /// Find font pairs using combined metric + visual similarity
    pub async fn find_font_pairs(
        &self,
        font_id: &str,
        limit: usize,
    ) -> Result<Vec<FontPairing>, String> {
        // Get target font details
        let points = self.client
            .get_points(
                qdrant_client::qdrant::GetPoints {
                    collection_name: COLLECTION_NAME.to_string(),
                    ids: vec![font_id.to_string().into()],
                    with_payload: Some(true.into()),
                    with_vectors: Some(true.into()),
                    ..Default::default()
                }
            )
            .await
            .map_err(|e| format!("Failed to get font: {}", e))?;

        if points.result.is_empty() {
            return Err("Font not found in Qdrant".to_string());
        }

        let target_point = &points.result[0];
        let target_payload = &target_point.payload;
        
        // Extract target metrics
        let target_category = get_string_field(target_payload, "primary_class").unwrap_or_default();
        let target_x_height = get_float_field(target_payload, "x_height").unwrap_or(0.5);
        let target_contrast = get_float_field(target_payload, "stroke_contrast").unwrap_or(1.0);
        let target_width = get_float_field(target_payload, "width_variance").unwrap_or(0.15);

        // Find more candidates (limit * 16) to ensure we find cross-category pairs
        let search_limit = (limit * 16).max(80);
        let similar = self.find_similar_by_metrics(font_id, search_limit).await?;

        // Score and rank for pairing
        let mut pairings: Vec<FontPairing> = similar
            .into_iter()
            .map(|candidate| {
                let candidate_category = candidate.primary_class.as_deref().unwrap_or("");
                let pairing_score = calculate_pairing_score(
                    &target_category,
                    target_x_height,
                    target_contrast,
                    target_width,
                    &candidate,
                    None,
                );

                let rationale = generate_pairing_rationale(
                    &target_category,
                    candidate_category,
                    target_x_height,
                    candidate.x_height,
                    pairing_score,
                );

                FontPairing {
                    font_id: candidate.font_id,
                    font_name: candidate.name,
                    file_path: candidate.file_path,
                    score: pairing_score,
                    visual_similarity: candidate.similarity_score,
                    metric_harmony: MetricHarmony {
                        x_height: 1.0 - (target_x_height - candidate.x_height).abs().min(1.0),
                        contrast: 1.0 - ((target_contrast - candidate.stroke_contrast).abs() / 5.0).min(1.0),
                        width: 1.0 - (target_width - candidate.width_variance).abs().min(1.0),
                    },
                    rationale,
                }
            })
            .collect();

        // Sort by pairing score
        pairings.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        pairings.truncate(limit);

        Ok(pairings)
    }

    /// Find font combinations using multiple pairing logics
    /// Returns results grouped by pairing algorithm
    pub async fn find_font_combos(
        &self,
        font_id: &str,
        limit_per_logic: usize,
    ) -> Result<Vec<PairingByLogic>, String> {
        // Get target font details
        let points = self.client
            .get_points(
                qdrant_client::qdrant::GetPoints {
                    collection_name: COLLECTION_NAME.to_string(),
                    ids: vec![font_id.to_string().into()],
                    with_payload: Some(true.into()),
                    with_vectors: Some(true.into()),
                    ..Default::default()
                }
            )
            .await
            .map_err(|e| format!("Failed to get font: {}", e))?;

        if points.result.is_empty() {
            return Err("Font not found in Qdrant".to_string());
        }

        let target_point = &points.result[0];
        let target_payload = &target_point.payload;
        let target_name = get_string_field(target_payload, "name").unwrap_or_default();
        let target_file_path = get_string_field(target_payload, "path").unwrap_or_default();
        
        // Extract target metrics for pairing
        let target_category = get_string_field(target_payload, "primary_class").unwrap_or_default();
        let target_x_height = get_float_field(target_payload, "x_height").unwrap_or(0.5);
        let target_contrast = get_float_field(target_payload, "stroke_contrast").unwrap_or(1.0);
        let target_width = get_float_field(target_payload, "width_variance").unwrap_or(0.15);

        // Fetch many candidates
        let candidates = self.find_similar_by_metrics(font_id, 100).await?;
        
        // Get target vectors for comparison
        let target_vectors = self.get_font_vectors(font_id).await.unwrap_or_default();
        let target_perceptual = target_vectors.get("perceptual");

        let mut logics: Vec<PairingByLogic> = Vec::new();

        // === LOGIC 1: Classic Contrast (Serif + Sans) ===
        let contrast_combos = self.build_contrast_combos(
            font_id, &target_name, &target_file_path, &target_category, target_x_height,
            target_contrast, target_width,
            target_perceptual.map(|v| v.as_slice()),
            &candidates, limit_per_logic
        ).await;
        logics.push(PairingByLogic {
            logic_name: "Classic Contrast".to_string(),
            logic_key: "contrast".to_string(),
            description: "Serif headlines with Sans body, or vice versa. The timeless pairing.".to_string(),
            results: contrast_combos,
        });

        // === LOGIC 2: Harmonic Unity (Same category, matching metrics) ===
        let harmony_combos = self.build_harmony_combos(
            font_id, &target_name, &target_file_path, &target_category, target_x_height,
            target_contrast, target_width,
            target_perceptual.map(|v| v.as_slice()),
            &candidates, limit_per_logic
        ).await;
        logics.push(PairingByLogic {
            logic_name: "Harmonic Unity".to_string(),
            logic_key: "harmony".to_string(),
            description: "Fonts with matching proportions for cohesive aesthetics.".to_string(),
            results: harmony_combos,
        });

        // === LOGIC 3: Modern Tech (Sans + Mono) ===
        let modern_combos = self.build_modern_combos(
            font_id, &target_name, &target_file_path, &target_category, target_x_height,
            target_contrast, target_width,
            target_perceptual.map(|v| v.as_slice()),
            &candidates, limit_per_logic
        ).await;
        logics.push(PairingByLogic {
            logic_name: "Modern Tech".to_string(),
            logic_key: "modern".to_string(),
            description: "Sans and Monospace for tech and developer aesthetics.".to_string(),
            results: modern_combos,
        });

        Ok(logics)
    }

    /// Build Classic Contrast combos (serif + sans)
    async fn build_contrast_combos(
        &self,
        target_id: &str, target_name: &str, target_path: &str, target_cat: &str, target_xh: f32,
        target_contrast: f32, target_width: f32,
        target_perceptual: Option<&[f32]>,
        candidates: &[SimilarFont], limit: usize
    ) -> Vec<FontCombo> {
        let t_cat = target_cat.to_lowercase();
        let opposite_cat = if t_cat == "serif" { "sans" } else if t_cat == "sans" { "serif" } else { "" };
        
        let mut combos: Vec<FontCombo> = Vec::new();
        
        for c in candidates.iter() {
            let c_cat = c.primary_class.as_deref().unwrap_or("").to_lowercase();
            if c_cat == opposite_cat {
                // Fetch candidate's perceptual vector for scoring if needed
                let c_perceptual_vec = self.get_font_vectors(&c.font_id).await.unwrap_or_default();
                let c_perceptual = c_perceptual_vec.get("perceptual");

                let perceptual_similarity = if let (Some(t_vec), Some(c_vec)) = (target_perceptual, c_perceptual) {
                    Some(cosine_similarity(t_vec, c_vec))
                } else {
                    None
                };

                let score = calculate_pairing_score(
                    target_cat, target_xh, target_contrast, target_width, c, perceptual_similarity
                );

                if combos.len() < limit {
                    let (headline_role, body_role) = if t_cat == "serif" {
                        ("headline", "body")
                    } else {
                        ("body", "headline")
                    };
                    combos.push(FontCombo {
                        fonts: vec![
                            FontComboEntry {
                                font_id: target_id.to_string(),
                                font_name: target_name.to_string(),
                                file_path: target_path.to_string(),
                                role: headline_role.to_string(),
                                primary_class: Some(target_cat.to_string()),
                            },
                            FontComboEntry {
                                font_id: c.font_id.clone(),
                                font_name: c.name.clone(),
                                file_path: c.file_path.clone(),
                                role: body_role.to_string(),
                                primary_class: c.primary_class.clone(),
                            },
                        ],
                        score,
                        rationale: format!("Classic {} + {} pairing with matched x-heights.", target_cat, opposite_cat),
                    });
                }
            }
        }
        combos.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        combos
    }

    /// Build Harmonic Unity combos (same category, similar metrics)
    async fn build_harmony_combos(
        &self,
        target_id: &str, target_name: &str, target_path: &str, target_cat: &str, target_xh: f32,
        target_contrast: f32, target_width: f32,
        target_perceptual: Option<&[f32]>,
        candidates: &[SimilarFont], limit: usize
    ) -> Vec<FontCombo> {
        let t_cat = target_cat.to_lowercase();
        
        let mut combos: Vec<FontCombo> = Vec::new();

        for c in candidates.iter() {
            let c_cat = c.primary_class.as_deref().unwrap_or("").to_lowercase();
            if c_cat == t_cat && c.font_id != target_id {
                let c_perceptual_vec = self.get_font_vectors(&c.font_id).await.unwrap_or_default();
                let c_perceptual = c_perceptual_vec.get("perceptual");

                let perceptual_similarity = if let (Some(t_vec), Some(c_vec)) = (target_perceptual, c_perceptual) {
                    Some(cosine_similarity(t_vec, c_vec))
                } else {
                    None
                };

                let score = calculate_pairing_score(
                    target_cat, target_xh, target_contrast, target_width, c, perceptual_similarity
                );

                if combos.len() < limit {
                    combos.push(FontCombo {
                        fonts: vec![
                            FontComboEntry {
                                font_id: target_id.to_string(),
                                font_name: target_name.to_string(),
                                file_path: target_path.to_string(),
                                role: "headline".to_string(),
                                primary_class: Some(target_cat.to_string()),
                            },
                            FontComboEntry {
                                font_id: c.font_id.clone(),
                                font_name: c.name.clone(),
                                file_path: c.file_path.clone(),
                                role: "body".to_string(),
                                primary_class: c.primary_class.clone(),
                            },
                        ],
                        score,
                        rationale: "Harmonious proportions create a cohesive reading experience.".to_string(),
                    });
                }
            }
        }
        combos.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        combos
    }

    async fn build_modern_combos(
        &self,
        target_id: &str, target_name: &str, target_path: &str, target_cat: &str, target_xh: f32,
        target_contrast: f32, target_width: f32,
        target_perceptual: Option<&[f32]>,
        candidates: &[SimilarFont], limit: usize
    ) -> Vec<FontCombo> {
        let t_cat = target_cat.to_lowercase();
        let partner_cat = if t_cat == "monospace" { "sans" } else if t_cat == "sans" { "monospace" } else { "" };
        
        let mut combos: Vec<FontCombo> = Vec::new();

        for c in candidates.iter() {
            let c_cat = c.primary_class.as_deref().unwrap_or("").to_lowercase();
            if c_cat == partner_cat {
                let c_perceptual_vec = self.get_font_vectors(&c.font_id).await.unwrap_or_default();
                let c_perceptual = c_perceptual_vec.get("perceptual");

                let perceptual_similarity = if let (Some(t_vec), Some(c_vec)) = (target_perceptual, c_perceptual) {
                    Some(cosine_similarity(t_vec, c_vec))
                } else {
                    None
                };

                let score = calculate_pairing_score(
                    target_cat, target_xh, target_contrast, target_width, c, perceptual_similarity
                );

                if combos.len() < limit {
                    let (primary_role, secondary_role) = if t_cat == "monospace" {
                        ("code", "ui")
                    } else {
                        ("ui", "code")
                    };
                    combos.push(FontCombo {
                        fonts: vec![
                            FontComboEntry {
                                font_id: target_id.to_string(),
                                font_name: target_name.to_string(),
                                file_path: target_path.to_string(),
                                role: primary_role.to_string(),
                                primary_class: Some(target_cat.to_string()),
                            },
                            FontComboEntry {
                                font_id: c.font_id.clone(),
                                font_name: c.name.clone(),
                                file_path: c.file_path.clone(),
                                role: secondary_role.to_string(),
                                primary_class: c.primary_class.clone(),
                            },
                        ],
                        score: score * 1.1, // Slight boost for niche category matching
                        rationale: "Modern tech aesthetic: clean UI with monospace code.".to_string(),
                    });
                }
            }
        }
        combos.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        combos
    }
}


/// Calculate pairing score based on typographic principles
fn calculate_pairing_score(
    target_category: &str,
    target_x_height: f32,
    target_contrast: f32,
    target_width: f32,
    candidate: &SimilarFont,
    perceptual_similarity: Option<f32>,
) -> f32 {
    let mut score: f32 = 0.5;

    // 1. Category logic
    let t_cat = target_category.to_lowercase();
    let c_cat = candidate.primary_class.as_deref().unwrap_or("").to_lowercase();

    if t_cat != c_cat && !t_cat.is_empty() && !c_cat.is_empty() {
        if (t_cat == "serif" && c_cat == "sans") || (t_cat == "sans" && c_cat == "serif") {
            score += 0.35; // Classic pairing bonus
        } else if (t_cat == "monospace" && c_cat == "sans") || (t_cat == "sans" && c_cat == "monospace") {
            score += 0.25; // Modern tech pairing
        } else {
            score += 0.15; // General variety
        }
    } else if t_cat == c_cat && !t_cat.is_empty() {
        score -= 0.10; // Slight penalty for same category if not intentional harmony
    }

    // 2. X-height harmony (Crucial for reading flow)
    let x_height_diff = (target_x_height - candidate.x_height).abs();
    if x_height_diff < 0.03 {
        score += 0.40; // Perfect harmony
    } else if x_height_diff < 0.07 {
        score += 0.20; 
    } else if x_height_diff > 0.12 {
        score -= 0.30; // Jarring
    }

    // 3. Perceptual Similarity Correction
    if let Some(sim) = perceptual_similarity {
        if sim > 0.96 {
            score -= 0.60; // Too similar! Likely duplicates or near-twins.
        } else if sim > 0.90 {
            score -= 0.30; // High similarity, risky for contrast
        } else if sim > 0.75 && sim < 0.85 {
            score += 0.15; // Sweet spot for "sister" fonts
        }
    }

    // 4. Stroke contrast difference (Visual separation)
    let contrast_diff = (target_contrast - candidate.stroke_contrast).abs();
    if contrast_diff > 0.3 {
        score += 0.15; // Good contrast
    }

    // 5. Width variance (Don't pair ultra-wide with ultra-narrow unless intended)
    let width_diff = (target_width - candidate.width_variance).abs();
    if width_diff < 0.1 {
        score += 0.1; // Width harmony
    }

    score.clamp(0.0, 1.0)
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    if v1.len() != v2.len() || v1.is_empty() {
        return 0.0;
    }
    let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let norm_v1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let norm_v2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    
    if norm_v1 == 0.0 || norm_v2 == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_v1 * norm_v2)
}

/// Generate a descriptive rationale for the pairing suggesting why it works
fn generate_pairing_rationale(
    target_cat: &str,
    candidate_cat: &str,
    target_xh: f32,
    candidate_xh: f32,
    score: f32,
) -> String {
    let cat_contrast = target_cat != candidate_cat && !target_cat.is_empty() && !candidate_cat.is_empty();
    let xh_harmony = (target_xh - candidate_xh).abs() < 0.05;

    if score > 0.85 {
        format!("Exceptional match. {} x-height harmony provides a seamless reading experience with strong {}/{} contrast.", 
            if xh_harmony { "Perfect" } else { "Good" }, target_cat, candidate_cat)
    } else if cat_contrast && xh_harmony {
        format!("Classic pairing. Balanced {} and {} with matching structural proportions.", target_cat, candidate_cat)
    } else if cat_contrast {
        format!("Strong contrast. The {} and {} combination creates clear hierarchy.", target_cat, candidate_cat)
    } else if xh_harmony {
        "Harmonious metrics. These fonts share similar proportions despite category similarities.".to_string()
    } else {
        "Functional pairing with compatible typographic traits.".to_string()
    }
}

/// Generate human-readable rationale for pairing
fn generate_rationale(target_cat: &str, candidate_cat: &str, score: f32) -> String {
    if score > 0.85 {
        "Superb typographic match".to_string()
    } else if target_cat != candidate_cat && score > 0.7 {
        "Classic category contrast".to_string()
    } else if score > 0.7 {
        "Excellent harmony".to_string()
    } else {
        "Good complementary pairing".to_string()
    }
}

/// Helper to extract string field from payload
fn get_string_field(payload: &HashMap<String, Value>, key: &str) -> Option<String> {
    match payload.get(key)?.kind.as_ref()? {
        qdrant_client::qdrant::value::Kind::StringValue(s) => Some(s.clone()),
        _ => None,
    }
}

/// Helper to extract float field from payload
fn get_float_field(payload: &HashMap<String, Value>, key: &str) -> Option<f32> {
    match payload.get(key)?.kind.as_ref()? {
        qdrant_client::qdrant::value::Kind::DoubleValue(v) => Some(*v as f32),
        qdrant_client::qdrant::value::Kind::IntegerValue(v) => Some(*v as f32),
        _ => None,
    }
}
