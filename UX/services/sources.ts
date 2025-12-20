/**
 * Font Sources Service
 * =====================
 * 
 * Manages font sources (local directories, Google Fonts, Adobe Fonts).
 */

import { isDesktopApp } from './tauri';

// Check if Tauri is available
const isTauri = typeof window !== 'undefined' && (
    (window as any).__TAURI__ ||
    (window as any).__TAURI_INTERNALS__
);

interface TauriInvoke {
    (cmd: string, args?: Record<string, unknown>): Promise<any>;
}

// Dynamic import for invoke
const getInvoke = async (): Promise<TauriInvoke | null> => {
    if (!isTauri) return null;
    try {
        const { invoke } = await import('@tauri-apps/api/core');
        return invoke as TauriInvoke;
    } catch {
        return null;
    }
};

// Source record from database
export interface Source {
    id: string;
    name: string;
    source_type: 'directory' | 'google_fonts' | 'adobe_fonts';
    path?: string;
    metadata?: any;
    date_added: string;
    font_count?: number;  // Computed field
}

/**
 * Create a new source
 */
export async function createSource(
    name: string,
    source_type: 'directory' | 'google_fonts' | 'adobe_fonts',
    path?: string
): Promise<Source> {
    const invoke = await getInvoke();
    if (!invoke) {
        throw new Error('Tauri required for source management');
    }

    return invoke('create_source', {
        name,
        sourceType: source_type,  // Tauri converts Rust snake_case to JS camelCase
        path: path || null,
    });
}

/**
 * Get all sources
 */
export async function getSources(): Promise<Source[]> {
    const invoke = await getInvoke();
    if (!invoke) {
        console.warn('Tauri not available');
        return [];
    }
    return invoke('get_sources');
}

/**
 * Delete a source by ID
 */
export async function deleteSource(sourceId: string): Promise<void> {
    const invoke = await getInvoke();
    if (!invoke) {
        throw new Error('Tauri required');
    }
    return invoke('delete_source', { source_id: sourceId });
}

/**
 * Get font count for a source
 */
export async function getFontCountBySource(sourceId: string): Promise<number> {
    const invoke = await getInvoke();
    if (!invoke) {
        return 0;
    }
    return invoke('get_font_count_by_source', { source_id: sourceId });
}

/**
 * Get fonts by source with pagination
 */
export async function getFontsBySource(
    sourceId: string,
    limit: number = 100,
    offset: number = 0
): Promise<any[]> {
    const invoke = await getInvoke();
    if (!invoke) {
        return [];
    }
    return invoke('get_fonts_by_source', {
        source_id: sourceId,
        limit,
        offset
    });
}

/**
 * Get sources with font counts
 */
export async function getSourcesWithCounts(): Promise<Source[]> {
    const sources = await getSources();

    // Populate font counts
    const sourcesWithCounts = await Promise.all(
        sources.map(async (source) => ({
            ...source,
            font_count: await getFontCountBySource(source.id)
        }))
    );

    return sourcesWithCounts;
}
