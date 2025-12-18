/**
 * Font Pairing Logic
 * ==================
 * 
 * Calculates pairing scores between fonts based on typographic principles.
 * 
 * Principles:
 * 1. Contrast: Pairing a Serif with a Sans-Serif often works well.
 * 2. X-Height: Signs of similar x-heights indicate good compatibility.
 * 3. Width: Contrasting widths (Condensed vs Normal) can be effective for hierarchy.
 */

import { FontData, FontCategory, FontPairing } from '../types';

/**
 * Calculate compatibility score (0-1) between two fonts
 */
export function calculatePairingScore(target: FontData, candidate: FontData): number {
    if (!target?.metrics || !candidate?.metrics) return 0;

    // 0. Base Score
    let score = 0.5;

    // 1. Category Contrast (30% weight)
    // We generally want different categories for high contrast pairings
    // e.g. Serif + Sans is classic. Sans + Sans is harder to pull off unless weights differ significantly.
    const isCategoryDifferent = target.category !== candidate.category;
    if (isCategoryDifferent) {
        // Boost for Serif + Sans combo
        if ((target.category === FontCategory.Serif && candidate.category === FontCategory.SansSerif) ||
            (target.category === FontCategory.SansSerif && candidate.category === FontCategory.Serif)) {
            score += 0.25;
        } else {
            score += 0.15;
        }
    } else {
        // Penalty for same category unless they are distinct in other ways (handled below)
        score -= 0.1;
    }

    // 2. X-Height Harmony (40% weight)
    // Fonts with similar x-height ratios relative to their cap height sit better together.
    const xHeightDiff = Math.abs(target.metrics.xHeightRatio - candidate.metrics.xHeightRatio);
    // x-height ratio is usually 0.4 - 0.6. A difference of < 0.05 is good.
    if (xHeightDiff < 0.03) score += 0.3;
    else if (xHeightDiff < 0.06) score += 0.15;
    else if (xHeightDiff > 0.1) score -= 0.1;

    // 3. Width Variance (20% weight)
    // If one is condensed, pairing with a normal/wide font is good.
    const widthDiff = Math.abs(target.metrics.widthVariance - candidate.metrics.widthVariance);
    // Width variance > 1.0 means variable widths. > 0.5 difference is significant
    if (widthDiff > 0.3) score += 0.15;

    // 4. Stroke Contrast (10% weight)
    // Mixing high contrast (didone) with low contrast (geometric sans)
    const contrastDiff = Math.abs(target.metrics.strokeContrast - candidate.metrics.strokeContrast);
    if (contrastDiff > 0.2) score += 0.1;

    // Clamp score 0-1
    return Math.max(0, Math.min(1, score));
}

export function generatePairings(targetFont: FontData, libraryFonts: FontData[]): FontPairing[] {
    if (!targetFont || !libraryFonts || !libraryFonts.length) return [];
    if (!targetFont.metrics) return [];

    // Filter out the font itself and ensure candidates have metrics
    const candidates = libraryFonts.filter(f => f.id !== targetFont.id && f.metrics);

    // Calculate scores
    const scored = candidates.map(candidate => {
        const score = calculatePairingScore(targetFont, candidate);

        let rationale = 'Good harmony';
        const isCategoryDifferent = targetFont.category !== candidate.category;
        const xHeightDiff = Math.abs(targetFont.metrics.xHeightRatio - candidate.metrics.xHeightRatio);

        if (isCategoryDifferent && score > 0.7) rationale = 'Classic category contrast';
        if (xHeightDiff < 0.03 && score > 0.7) rationale = 'Excellent x-height alignment';
        if (score > 0.85) rationale = 'Superb typographic match';

        return {
            targetFontId: candidate.id,
            font: candidate, // Helper to include actual object
            score,
            rationale,
            metrics_harmony: {
                xHeight: 1 - Math.min(1, xHeightDiff * 5), // Normalized 0-1
                contrast: Math.min(1, Math.abs(targetFont.metrics.strokeContrast - candidate.metrics.strokeContrast)),
                width: Math.min(1, Math.abs(targetFont.metrics.widthVariance - candidate.metrics.widthVariance))
            }
        };
    });

    // Sort by score desc
    return scored
        .sort((a, b) => b.score - a.score)
        .slice(0, 5)
        .map(({ font, ...rest }) => rest); // Return strictly FontPairing interface (though we might want to extend it)
}
