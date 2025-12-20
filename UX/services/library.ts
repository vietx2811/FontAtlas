/**
 * Font Library Service
 * =====================
 * 
 * Manages local font library using Tauri SQLite database.
 */

import { isDesktopApp } from './tauri';

// Check if Tauri is available (works for both v1 and v2)
const isTauri = typeof window !== 'undefined' && (
    (window as any).__TAURI__ ||
    (window as any).__TAURI_INTERNALS__
);

interface TauriInvoke {
    (cmd: string, args?: Record<string, unknown>): Promise<any>;
}

// Dynamic import for invoke - works with Tauri v2
const getInvoke = async (): Promise<TauriInvoke | null> => {
    if (!isTauri) return null;
    try {
        const { invoke } = await import('@tauri-apps/api/core');
        return invoke as TauriInvoke;
    } catch {
        return null;
    }
};

// Font record from database
export interface LibraryFont {
    id: string;
    name: string;
    file_path: string;
    original_directory: string;
    category: string | null;
    classification: string | null;
    tags: string[];
    collection_id: string | null;
    date_added: string;
    date_modified: string;
    is_synced: boolean;
    file_exists: boolean;
    x_height: number | null;
    width_variance: number | null;
    stroke_contrast: number | null;
    aperture: number | null;
    foundry: string | null;
    weight: number;
    is_italic: boolean;
    supports_vietnamese: boolean;
    family_name: string | null;
    subfamily: string | null;
    weight_class: number | null;
    width_class: number | null;
    source_id: string | null;
    // Enhanced taxonomy (new classification system)
    primary_class: string | null;
    secondary_tags: string | null;  // JSON string of array
    serif_subtype: string | null;
    sans_subtype: string | null;
    script_subtype: string | null;
}

// Collection (virtual folder)
export interface Collection {
    id: string;
    name: string;
    color: string | null;
    created_at: string;
}

/**
 * Get all fonts from library
 */
export async function getLibraryFonts(limit: number = 100, offset: number = 0, category?: string): Promise<LibraryFont[]> {
    const invoke = await getInvoke();
    if (!invoke) {
        console.warn('Tauri not available');
        return [];
    }
    return invoke('get_library_fonts', { limit, offset, category });
}

/**
 * Search fonts in library
 */
export async function searchLibrary(query: string): Promise<LibraryFont[]> {
    const invoke = await getInvoke();
    if (!invoke) {
        return [];
    }
    return invoke('search_library', { query });
}

/**
 * Add font to library
 */
export async function addFontToLibrary(
    name: string,
    filePath: string,
    category?: string,
    classification?: string,
    xHeight?: number,
    widthVariance?: number,
    strokeContrast?: number,
    aperture?: number,
    terminalSharpness?: number,
    stressAngle?: number,
    curvatureRatio?: number,
    counterDensity?: number,
    foundry?: string,
    weight?: number,
    isItalic?: boolean,
    supportsVietnamese?: boolean,
    familyName?: string,
    subfamily?: string,
    weightClass?: number,
    widthClass?: number,
    sourceId?: string,
    primaryClass?: string,
    secondaryTags?: string[],  // Array that will be serialized to JSON
    serifSubtype?: string,
    sansSubtype?: string,
    scriptSubtype?: string
): Promise<LibraryFont> {
    const invoke = await getInvoke();
    if (!invoke) {
        throw new Error('Tauri required for font library');
    }

    return invoke('add_font_to_library', {
        name,
        filePath,
        category: category || null,
        classification: classification || null,
        xHeight: xHeight || null,
        widthVariance: widthVariance || null,
        strokeContrast: strokeContrast || null,
        aperture: aperture || null,
        terminalSharpness: terminalSharpness || null,
        stressAngle: stressAngle || null,
        curvatureRatio: curvatureRatio || null,
        counterDensity: counterDensity || null,
        foundry: foundry || null,
        weight: weight || null,
        isItalic: isItalic || false,
        supportsVietnamese: supportsVietnamese || false,
        familyName: familyName || null,
        subfamily: subfamily || null,
        weightClass: weightClass || null,
        widthClass: widthClass || null,
        sourceId: sourceId || null,
        primaryClass: primaryClass || null,
        secondaryTags: secondaryTags ? JSON.stringify(secondaryTags) : null,
        serifSubtype: serifSubtype || null,
        sansSubtype: sansSubtype || null,
        scriptSubtype: scriptSubtype || null,
    });
}

/**
 * Remove font from library
 */
export async function removeFontFromLibrary(fontId: string): Promise<void> {
    const invoke = await getInvoke();
    if (!invoke) {
        throw new Error('Tauri required');
    }
    return invoke('remove_font_from_library', { font_id: fontId });
}

/**
 * Get font count
 */
export async function getFontCount(): Promise<number> {
    const invoke = await getInvoke();
    if (!invoke) {
        return 0;
    }
    return invoke('get_font_count');
}

/**
 * Clear all fonts from library
 */
export async function clearLibrary(): Promise<number> {
    const invoke = await getInvoke();
    if (!invoke) {
        throw new Error('Tauri required');
    }
    return invoke('clear_library');
}

/**
 * Get all collections
 */
export async function getCollections(): Promise<Collection[]> {
    const invoke = await getInvoke();
    if (!invoke) {
        return [];
    }
    return invoke('get_collections');
}

/**
 * Scan directory and add all fonts to library
 */
export async function scanAndAddDirectory(directoryPath: string, onProgress?: (msg: string) => void): Promise<number> {
    const invoke = await getInvoke();
    if (!invoke) {
        throw new Error('Tauri required');
    }

    if (onProgress) onProgress('Listing files...');

    // Create source for this directory
    const { createSource } = await import('./sources');
    const folderName = directoryPath.split('/').pop() || 'Unknown Folder';
    const source = await createSource(folderName, 'directory', directoryPath);
    console.log('Created source:', source);

    // Scan for fonts
    const fonts = await invoke('scan_directory', { path: directoryPath }) as any[];
    console.log('Scanned fonts:', fonts);

    if (onProgress) onProgress(`Found ${fonts.length} files. Processing...`);

    let added = 0;
    let cloudOnly = 0;
    let processed = 0;
    const total = fonts.length;

    for (const font of fonts) {
        processed++;
        // Skip cloud-only fonts for now (not locally available)
        if (!font.is_local) {
            console.log('Cloud-only font (not downloaded):', font.name);
            cloudOnly++;
            if (onProgress) onProgress(`Skipping cloud font (${processed}/${total}): ${font.name}`);
            continue;
        }

        if (onProgress) onProgress(`Importing (${processed}/${total}): ${font.name}`);

        try {
            // Try to parse with sidecar (optional)
            let parsed: any = null;
            try {
                parsed = await invoke('parse_font_with_sidecar', { fontPath: font.path });
                console.log('Parsed font metrics:', font.name, parsed?.basic_metrics);
            } catch (e) {
                console.warn('Sidecar parsing failed for', font.name, '- adding without metrics');
            }

            // Add to library with or without parsed metrics
            await addFontToLibrary(
                parsed?.name || font.name.replace(/\.(ttf|otf|woff|woff2)$/i, ''),
                font.path,
                undefined,
                parsed?.classification,
                parsed?.anatomy_metrics?.x_height,
                parsed?.anatomy_metrics?.width_variance,
                parsed?.anatomy_metrics?.stroke_contrast,
                parsed?.anatomy_metrics?.aperture,
                parsed?.anatomy_metrics?.terminal_sharpness,
                parsed?.anatomy_metrics?.stress_angle,
                parsed?.anatomy_metrics?.curvature_ratio,
                parsed?.anatomy_metrics?.counter_density,
                parsed?.foundry,
                parsed?.weight,
                parsed?.is_italic,
                parsed?.supports_vietnamese,
                parsed?.family_name,
                parsed?.subfamily,
                parsed?.weight_class,
                parsed?.width_class,
                source.id,
                // Enhanced taxonomy (Phase 4)
                parsed?.primary_class,
                parsed?.secondary_tags,  // Array from sidecar
                parsed?.serif_subtype,
                parsed?.sans_subtype,
                parsed?.script_subtype
            );
            added++;
        } catch (e) {
            console.error('Failed to add font:', font.name, e);
        }
    }

    if (cloudOnly > 0) {
        console.log(`Skipped ${cloudOnly} cloud-only fonts. Download them in Finder to add.`);
    }

    return added;
}
