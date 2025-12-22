/**
 * useFontLoader Hook
 * ===================
 * 
 * Loads font files dynamically using Tauri's FS plugin and FontFace API.
 * Uses Blob URLs to avoid 'unsupported URL' errors while maintaining performance
 * through lazy loading (isVisible).
 */

import { useEffect, useState } from 'react';
import { readFile } from '@tauri-apps/plugin-fs';

export function useFontLoader(fontId: string, filePath?: string, isVisible: boolean = true): {
    fontFamily: string;
    isLoading: boolean;
} {
    const [fontFamily, setFontFamily] = useState('sans-serif');
    const [isLoading, setIsLoading] = useState(false);
    const [isLoaded, setIsLoaded] = useState(false);

    useEffect(() => {
        // 1. Reset/Skip if not visible
        if (!isVisible) {
            return;
        }

        let mounted = true;
        let objectUrl: string | null = null;

        const fontFaceName = `font-${fontId}`;

        // 2. Check if already loaded
        if (checkFontLoaded(fontFaceName)) {
            if (mounted) {
                setFontFamily(fontFaceName);
                setIsLoaded(true);
            }
            return;
        }

        if (!filePath) return;

        const loadFont = async () => {
            setIsLoading(true);

            try {
                // 3. Read file content
                const bytes = await readFile(filePath);

                if (!mounted) return;

                // 4. Create Blob URL
                const mimeType = getMimeType(filePath);
                const blob = new Blob([bytes], { type: mimeType });
                objectUrl = URL.createObjectURL(blob);

                // 5. Load FontFace
                const fontFace = new FontFace(fontFaceName, `url("${objectUrl}")`);
                await fontFace.load();
                document.fonts.add(fontFace);

                if (mounted) {
                    setFontFamily(fontFaceName);
                    setIsLoading(false);
                    setIsLoaded(true);
                }

            } catch (e) {
                console.warn(`Failed to load font ${fontId} from ${filePath}:`, e);
                if (mounted) setIsLoading(false);
            }
        };

        loadFont();

        // Cleanup
        return () => {
            mounted = false;
            // Revoke immediately after loading to save memory, 
            // relying on document.fonts to hold the data.
            // Note: Keeping Object URL alive to ensuring rendering stability in Tauri.
            // Revoking it too early can cause the font to disappear or fail to render.
            // if (objectUrl) {
            //    URL.revokeObjectURL(objectUrl);
            // }
        };
    }, [fontId, filePath, isVisible]);

    return {
        fontFamily: isLoaded ? fontFamily : 'sans-serif',
        isLoading: isVisible && !isLoaded
    };
}

function checkFontLoaded(fontFamily: string): boolean {
    return Array.from(document.fonts).some(f => f.family === fontFamily && f.status === 'loaded');
}

function getMimeType(path: string): string {
    const ext = path.toLowerCase().split('.').pop();
    switch (ext) {
        case 'otf': return 'font/otf';
        case 'woff': return 'font/woff';
        case 'woff2': return 'font/woff2';
        default: return 'font/ttf';
    }
}
