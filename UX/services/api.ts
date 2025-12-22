/**
 * Base API Client
 * ================
 * 
 * Handles all HTTP requests to the FastAPI backend.
 * Provides consistent error handling and response formatting.
 */

// Connect directly to FastAPI backend (bypass Vite proxy)
const API_BASE_URL = 'http://localhost:8000/api';

export class ApiError extends Error {
    constructor(
        message: string,
        public status: number,
        public response?: any
    ) {
        super(message);
        this.name = 'ApiError';
    }
}

export class ApiClient {
    private baseUrl: string;

    constructor(baseUrl: string = API_BASE_URL) {
        this.baseUrl = baseUrl;
    }

    /**
     * Generic GET request
     */
    async get<T>(endpoint: string, params?: Record<string, any>): Promise<T> {
        // Construct full URL - handle both relative and absolute baseUrl
        const fullUrl = endpoint.startsWith('http')
            ? endpoint
            : `${this.baseUrl}${endpoint}`;

        const url = new URL(fullUrl);

        if (params) {
            Object.entries(params).forEach(([key, value]) => {
                if (value !== undefined && value !== null) {
                    url.searchParams.append(key, String(value));
                }
            });
        }

        const response = await fetch(url.toString(), {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
            },
        });

        return this.handleResponse<T>(response);
    }

    /**
     * Generic POST request
     */
    async post<T>(endpoint: string, body?: any): Promise<T> {
        const fullUrl = endpoint.startsWith('http')
            ? endpoint
            : `${this.baseUrl}${endpoint}`;

        const response = await fetch(fullUrl, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: body ? JSON.stringify(body) : undefined,
        });

        return this.handleResponse<T>(response);
    }

    /**
     * Handle API response
     */
    private async handleResponse<T>(response: Response): Promise<T> {
        if (!response.ok) {
            const error = await response.json().catch(() => ({
                detail: response.statusText
            }));

            throw new ApiError(
                error.detail || 'API request failed',
                response.status,
                error
            );
        }

        return response.json();
    }
}

// Singleton instance
export const apiClient = new ApiClient();
