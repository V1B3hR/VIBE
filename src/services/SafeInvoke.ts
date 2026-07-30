import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { logInvokeError } from './ErrorLogger';

export interface InvokeOptions {
    showErrorToast?: boolean;
    errorMessage?: string;
    suppressLogging?: boolean;
    retryCount?: number;
    retryDelay?: number;
}

export async function safeInvoke<T = unknown>(
    command: string,
    params?: Record<string, unknown>,
    options: InvokeOptions = {}
): Promise<T | null> {
    const {
        showErrorToast = false,
        errorMessage,
        suppressLogging = false,
        retryCount = 0,
        retryDelay = 1000
    } = options;

    let lastError: unknown;
    let attempts = 0;
    const maxAttempts = retryCount + 1;

    while (attempts < maxAttempts) {
        try {
            const result = await tauriInvoke<T>(command, params);
            return result;
        } catch (error) {
            lastError = error;
            attempts++;

            if (!suppressLogging) {
                logInvokeError(command, params || {}, error);
            }

            if (attempts < maxAttempts) {
                console.warn(`Retrying ${command} (attempt ${attempts + 1}/${maxAttempts})...`);
                await sleep(retryDelay);
                continue;
            }

            if (showErrorToast) {
                const message = errorMessage || `Failed to execute: ${command}`;
                const errorDetails = error instanceof Error ? error.message : String(error);
                console.error(`${message}: ${errorDetails}`);
            }

            return null;
        }
    }

    return null;
}

export async function unsafeInvoke<T = unknown>(
    command: string,
    params?: Record<string, unknown>
): Promise<T> {
    return await tauriInvoke<T>(command, params);
}

export async function invokeWithRetry<T = unknown>(
    command: string,
    params?: Record<string, unknown>,
    maxRetries: number = 3,
    delayMs: number = 1000
): Promise<T | null> {
    return safeInvoke<T>(command, params, {
        retryCount: maxRetries,
        retryDelay: delayMs
    });
}

export async function invokeWithToast<T = unknown>(
    command: string,
    params?: Record<string, unknown>,
    errorMessage?: string
): Promise<T | null> {
    return safeInvoke<T>(command, params, {
        showErrorToast: true,
        errorMessage
    });
}

function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export { tauriInvoke as invoke };
