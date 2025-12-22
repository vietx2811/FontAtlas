/**
 * Scanning Service
 * =================
 * 
 * API calls for scanning and indexing operations
 */

import { apiClient } from './api';

export interface ScanRequest {
    directory: string;
    force: boolean;
}

export interface ScanResponse {
    task_id: string;
    message: string;
    directory: string;
}

export interface ScanProgress {
    total: number;
    current: number;
    status: 'idle' | 'scanning' | 'complete' | 'error';
    current_file: string;
    current_step: string;
    errors: string[];
}

export interface IndexResponse {
    task_id: string;
    message: string;
    directory: string;
}

export interface IndexProgress {
    total: number;
    current: number;
    status: 'idle' | 'indexing' | 'complete' | 'error';
    current_file: string;
    errors: string[];
}

export const scanningService = {
    /**
     * Start scanning directory for fonts
     */
    async startScan(directory: string, force: boolean = false): Promise<ScanResponse> {
        return apiClient.post<ScanResponse>('/fonts/scan', { directory, force });
    },

    /**
     * Get scan progress
     */
    async getScanProgress(taskId: string): Promise<ScanProgress> {
        return apiClient.get<ScanProgress>(`/fonts/scan/${taskId}/progress`);
    },

    /**
     * Start indexing fonts to Qdrant
     */
    async startIndex(directory: string): Promise<IndexResponse> {
        return apiClient.post<IndexResponse>('/fonts/index', { directory });
    },

    /**
     * Get index progress
     */
    async getIndexProgress(taskId: string): Promise<IndexProgress> {
        return apiClient.get<IndexProgress>(`/fonts/index/${taskId}/progress`);
    },
};
