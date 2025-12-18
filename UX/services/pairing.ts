/**
 * Pairing Service
 * ================
 * 
 * API calls for font pairing operations
 */

import { apiClient } from './api';

export interface PairingRequest {
    font_id: string;
    limit: number;
}

export interface ComponentScores {
    x_height_score: number;
    width_variance_score: number;
    stroke_contrast_score: number;
    aperture_score: number;
    perceptual_score: number;
    category_bonus: number;
}

export interface PairingResult {
    font_id: string;
    font_name: string;
    score: number;
    qdrant_similarity: number;
    components: ComponentScores;
    explanation?: string;
}

export interface PairingResponse {
    query_font: string;
    pairs: PairingResult[];
}

export const pairingService = {
    /**
     * Find complementary font pairs
     */
    async findPairs(
        fontId: string,
        directory: string,
        limit: number = 5
    ): Promise<PairingResponse> {
        return apiClient.post<PairingResponse>(
            `/pairs/find?directory=${encodeURIComponent(directory)}`,
            { font_id: fontId, limit }
        );
    },
};
