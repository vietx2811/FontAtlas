import { invoke } from '@tauri-apps/api/core';

export interface SimilarFont {
    font_id: string;
    name: string;
    file_path: string;
    similarity_score: number;
    category?: string;
    primary_class?: string;
    foundry?: string;
    x_height: number;
    width_variance: number;
    stroke_contrast: number;
    aperture: number;
}

export interface MetricHarmony {
    x_height: number;
    contrast: number;
    width: number;
}

export interface FontPairingResult {
    font_id: string;
    font_name: string;
    file_path: string;
    score: number;
    visual_similarity: number;
    metric_harmony: MetricHarmony;
    rationale: string;
}

// ============================================================================
// Enhanced Pairing Types (2-3 font combos, multiple logics)
// ============================================================================

export interface FontComboEntry {
    font_id: string;
    font_name: string;
    file_path: string;
    role: string;  // "headline", "body", "accent", "code", "ui"
    primary_class?: string;
}

export interface FontCombo {
    fonts: FontComboEntry[];
    score: number;
    rationale: string;
}

export interface PairingByLogic {
    logic_name: string;   // "Classic Contrast", "Harmonic Unity", "Modern Tech"
    logic_key: string;    // "contrast", "harmony", "modern"
    description: string;
    results: FontCombo[];
}

export const qdrantService = {
    /**
     * Find similar fonts using vector similarity search
     */
    async findSimilarFonts(fontId: string, limit: number = 5, mode: 'metrics' | 'perceptual' = 'perceptual'): Promise<SimilarFont[]> {
        try {
            const result = await invoke<SimilarFont[]>('find_similar_fonts', {
                fontId,
                limit,
                mode
            });

            return result;
        } catch (error) {
            console.error('Failed to find similar fonts:', error);
            throw error;
        }
    },

    /**
     * Find complementary font pairs
     */
    async findFontPairs(fontId: string, limit: number = 5): Promise<FontPairingResult[]> {
        try {
            const result = await invoke<FontPairingResult[]>('find_font_pairs', {
                fontId,
                limit
            });

            return result;
        } catch (error) {
            console.error('Failed to find font pairs:', error);
            throw error;
        }
    },

    /**
     * Find font combinations using multiple pairing logics
     */
    async findFontCombos(fontId: string, limitPerLogic: number = 3): Promise<PairingByLogic[]> {
        try {
            const result = await invoke<PairingByLogic[]>('find_font_combos', {
                fontId,
                limitPerLogic
            });

            return result;
        } catch (error) {
            console.error('Failed to find font combos:', error);
            throw error;
        }
    }
};
