/**
 * useFonts Hook
 * ==============
 * 
 * Hook for fetching and managing fonts from API
 */

import { useState, useEffect } from 'react';
import { fontService, type Font, type FontListResponse } from '../services/fonts';

export interface UseFontsResult {
    fonts: Font[];
    isLoading: boolean;
    error: Error | null;
    refetch: () => void;
}

export function useFonts(directory: string): UseFontsResult {
    const [fonts, setFonts] = useState<Font[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<Error | null>(null);

    const fetchFonts = async () => {
        if (!directory) {
            setFonts([]);
            setIsLoading(false);
            return;
        }

        try {
            setIsLoading(true);
            setError(null);
            const response: FontListResponse = await fontService.listFonts(directory);
            setFonts(response.fonts);
        } catch (err) {
            setError(err as Error);
            console.error('Failed to fetch fonts:', err);
        } finally {
            setIsLoading(false);
        }
    };

    useEffect(() => {
        fetchFonts();
    }, [directory]);

    return {
        fonts,
        isLoading,
        error,
        refetch: fetchFonts,
    };
}
