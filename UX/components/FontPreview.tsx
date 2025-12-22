/**
 * Font Preview Component
 * =======================
 * 
 * Dynamically loads font files and renders preview text with @font-face
 */

import React, { useEffect, useState, useMemo } from 'react';

interface FontPreviewProps {
    fontPath: string;
    fontName: string;
    text?: string;
    size?: number;
    className?: string;
}

// Cache for loaded font URLs
const fontUrlCache = new Map<string, string>();

// Check if Tauri is available
const isTauri = typeof window !== 'undefined' && (window as any).__TAURI__;

/**
 * Load font file and create object URL for @font-face
 */
async function loadFontUrl(fontPath: string): Promise<string> {
    // Check cache first
    if (fontUrlCache.has(fontPath)) {
        return fontUrlCache.get(fontPath)!;
    }

    if (isTauri) {
        // Use Tauri to read font file
        const invoke = (window as any).__TAURI__.core.invoke;
        const bytes: number[] = await invoke('read_font_file', { path: fontPath });
        const uint8Array = new Uint8Array(bytes);
        const blob = new Blob([uint8Array], { type: 'font/ttf' });
        const url = URL.createObjectURL(blob);
        fontUrlCache.set(fontPath, url);
        return url;
    } else {
        // Web fallback - try to load from public fonts directory
        const fileName = fontPath.split('/').pop();
        const url = `/fonts/${fileName}`;
        fontUrlCache.set(fontPath, url);
        return url;
    }
}

/**
 * Create a unique font-family name from path
 */
function getFontFamilyName(fontPath: string): string {
    const base = fontPath.split('/').pop()?.replace(/\.(ttf|otf|woff|woff2)$/i, '') || 'preview';
    return `preview-${base}-${fontPath.length}`;
}

export const FontPreview: React.FC<FontPreviewProps> = ({
    fontPath,
    fontName,
    text = "The quick brown fox jumps over the lazy dog",
    size = 24,
    className = ""
}) => {
    const [fontUrl, setFontUrl] = useState<string | null>(null);
    const [isLoaded, setIsLoaded] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const fontFamily = useMemo(() => getFontFamilyName(fontPath), [fontPath]);

    useEffect(() => {
        let isMounted = true;

        async function loadFont() {
            try {
                const url = await loadFontUrl(fontPath);
                if (!isMounted) return;

                // Create @font-face rule
                const fontFace = new FontFace(fontFamily, `url(${url})`);
                await fontFace.load();
                document.fonts.add(fontFace);

                setFontUrl(url);
                setIsLoaded(true);
                setError(null);
            } catch (e) {
                if (isMounted) {
                    setError(e instanceof Error ? e.message : 'Failed to load font');
                    setIsLoaded(false);
                }
            }
        }

        if (fontPath) {
            loadFont();
        }

        return () => {
            isMounted = false;
        };
    }, [fontPath, fontFamily]);

    if (error) {
        return (
            <div className={`font-preview font-preview--error ${className}`}>
                <span style={{ color: '#ff6b6b', fontSize: 14 }}>
                    ⚠️ Failed to load: {fontName}
                </span>
            </div>
        );
    }

    if (!isLoaded) {
        return (
            <div className={`font-preview font-preview--loading ${className}`}>
                <span style={{ color: '#888', fontSize: 14 }}>Loading {fontName}...</span>
            </div>
        );
    }

    return (
        <div className={`font-preview ${className}`}>
            <p
                style={{
                    fontFamily: fontFamily,
                    fontSize: size,
                    lineHeight: 1.4,
                    margin: 0,
                }}
            >
                {text}
            </p>
        </div>
    );
};

/**
 * Waterfall Preview - shows font at multiple sizes
 */
interface WaterfallPreviewProps {
    fontPath: string;
    fontName: string;
    text?: string;
    sizes?: number[];
    className?: string;
}

export const WaterfallPreview: React.FC<WaterfallPreviewProps> = ({
    fontPath,
    fontName,
    text = "The quick brown fox",
    sizes = [12, 16, 20, 24, 32, 48, 64],
    className = ""
}) => {
    return (
        <div className={`waterfall-preview ${className}`}>
            {sizes.map(size => (
                <div key={size} style={{ marginBottom: 12 }}>
                    <span style={{ fontSize: 10, color: '#888', marginRight: 8 }}>{size}px</span>
                    <FontPreview
                        fontPath={fontPath}
                        fontName={fontName}
                        text={text}
                        size={size}
                    />
                </div>
            ))}
        </div>
    );
};

/**
 * Character Map Preview
 */
interface CharacterMapProps {
    fontPath: string;
    fontName: string;
    size?: number;
    className?: string;
}

const CHAR_RANGES = {
    uppercase: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
    lowercase: 'abcdefghijklmnopqrstuvwxyz',
    numbers: '0123456789',
    punctuation: '.,;:!?\'"-()[]{}@#$%&*',
};

export const CharacterMap: React.FC<CharacterMapProps> = ({
    fontPath,
    fontName,
    size = 32,
    className = ""
}) => {
    const fontFamily = useMemo(() => getFontFamilyName(fontPath), [fontPath]);
    const [isLoaded, setIsLoaded] = useState(false);

    useEffect(() => {
        loadFontUrl(fontPath)
            .then(url => {
                const fontFace = new FontFace(fontFamily, `url(${url})`);
                return fontFace.load();
            })
            .then(fontFace => {
                document.fonts.add(fontFace);
                setIsLoaded(true);
            })
            .catch(console.error);
    }, [fontPath, fontFamily]);

    if (!isLoaded) {
        return <div>Loading character map...</div>;
    }

    return (
        <div className={`character-map ${className}`}>
            {Object.entries(CHAR_RANGES).map(([name, chars]) => (
                <div key={name} style={{ marginBottom: 16 }}>
                    <div style={{ fontSize: 12, color: '#888', marginBottom: 8, textTransform: 'capitalize' }}>
                        {name}
                    </div>
                    <div
                        style={{
                            fontFamily,
                            fontSize: size,
                            letterSpacing: 4,
                            lineHeight: 1.5,
                        }}
                    >
                        {chars.split('').map((char, i) => (
                            <span key={i} style={{ display: 'inline-block', width: size * 1.2, textAlign: 'center' }}>
                                {char}
                            </span>
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
};

export default FontPreview;
