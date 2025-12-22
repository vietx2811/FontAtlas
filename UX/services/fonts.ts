/**
 * Font Service
 * =============
 * 
 * API calls for font operations:
 * - List fonts
 * - Get font details
 * - Get directories
 */

import { apiClient } from './api';

export interface FontMetrics {
    x_height_ratio: number;
    optical_x_ratio: number;
    width_variance: number;
    stroke_contrast: number;
    aperture: number;
    cap_height: number;
    x_height: number;
    units_per_em: number;
}

export interface Font {
    id: string;
    name: string;
    category: string;
    x_height: number;
    width_variance: number;
    stroke_contrast: number;
    aperture: number;
}

export interface FontDetail {
    metadata: {
        font_name: string;
        file_path: string;
    };
    basic_metrics: {
        x_height: number;
        cap_height: number;
        x_height_ratio: number;
        optical_x_ratio: number;
        x_height_source: string;
        width_variance: number;
        width_mean: number;
        units_per_em: number;
    };
    anatomy_metrics: {
        stroke_contrast: {
            mean_contrast: number;
            classification: string;
        };
        aperture_openness: {
            mean_aperture: number;
            classification: string;
        };
    };
    classification: {
        auto_category: string;
        confidence: number;
        reasoning: string;
        google_fonts_category?: string;
    };
    unicode_coverage: {
        total_glyphs: number;
        supports_vietnamese: boolean;
        supports_latin_extended: boolean;
    };
}

export interface FontListResponse {
    fonts: Font[];
    count: number;
}

export interface Directory {
    path: string;
    absolute_path: string;
    ttf_files: number;
    profile_files: number;
}

export interface DirectoriesResponse {
    directories: Directory[];
}

export const fontService = {
    /**
     * List fonts in a directory
     */
    async listFonts(directory: string): Promise<FontListResponse> {
        return apiClient.get<FontListResponse>('/fonts/', { directory });
    },

    /**
     * Get detailed font profile
     */
    async getFontDetail(fontId: string, directory: string): Promise<FontDetail> {
        return apiClient.get<FontDetail>(`/fonts/${fontId}`, { directory });
    },

    /**
     * List available directories
     */
    async getDirectories(): Promise<DirectoriesResponse> {
        return apiClient.get<DirectoriesResponse>('/fonts/directories');
    },

    /**
     * Get font file URL for preview
     */
    getFontFileUrl(fontId: string, directory: string): string {
        const params = new URLSearchParams({ directory });
        return `/api/fonts/font-file/${encodeURIComponent(fontId)}?${params}`;
    },
};
