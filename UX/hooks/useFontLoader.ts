/**
 * useFontLoader Hook
 * ===================
 * 
 * Loads font files dynamically using Tauri's FS plugin and FontFace API.
 * Caches loaded fonts to avoid reloading.
 */

import { useEffect, useState } from 'react';

// Global cache for loaded fonts
const fontCache = new Map<string, string>();

export function useFontLoader(fontId: string, filePath?: string): {
    fontFamily: string;
    isLoading: boolean;
} {
    const [fontFamily, setFontFamily] = useState('sans-serif');
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        let mounted = true;

        const loadFont = async () => {
            const cacheKey = `font-${fontId}`;

            // Check cache first
            if (fontCache.has(cacheKey)) {
                if (mounted) {
                    setFontFamily(cacheKey);
                    setIsLoading(false);
                }
                return;
            }

            try {
                // Check if we have a file path
                if (!filePath) {
                    if (mounted) setIsLoading(false);
                    return;
                }

                // Check if running in Tauri
                const isTauri = (window as any).__TAURI__ || (window as any).__TAURI_INTERNALS__;

                if (!isTauri) {
                    // Browser mode - can't load local fonts
                    if (mounted) setIsLoading(false);
                    return;
                }

                // Read font file as binary
                const { readFile } = await import('@tauri-apps/plugin-fs');
                const bytes = await readFile(filePath);

                // Determine MIME type
                const ext = filePath.toLowerCase().split('.').pop();
                let mimeType = 'font/ttf';
                if (ext === 'otf') mimeType = 'font/otf';
                else if (ext === 'woff') mimeType = 'font/woff';
                else if (ext === 'woff2') mimeType = 'font/woff2';

                // Create blob URL
                const blob = new Blob([bytes], { type: mimeType });
                const blobUrl = URL.createObjectURL(blob);

                // Load via FontFace API
                const fontFace = new FontFace(cacheKey, `url("${blobUrl}")`);
                await fontFace.load();
                document.fonts.add(fontFace);

                // Cache for reuse
                fontCache.set(cacheKey, blobUrl);

                if (mounted) {
                    setFontFamily(cacheKey);
                    setIsLoading(false);
                }
            } catch (e) {
                console.warn('Failed to load font:', fontId, e);
                if (mounted) setIsLoading(false);
            }
        };

        loadFont();

        return () => { mounted = false; };
    }, [fontId, filePath]);

    return { fontFamily, isLoading };
}

/**
 * Check if a font is already cached
 */
export function isFontCached(fontId: string): boolean {
    return fontCache.has(`font-${fontId}`);
}

/**
 * Get cached font family name
 */
export function getCachedFontFamily(fontId: string): string | null {
    const key = `font-${fontId}`;
    return fontCache.has(key) ? key : null;
}
