import { ErrorInfo } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface ErrorLog {
    timestamp: number;
    message: string;
    stack?: string;
    componentStack?: string;
    userAgent: string;
    url: string;
}

class ErrorLoggerService {
    private errorQueue: ErrorLog[] = [];
    private maxQueueSize = 50;

    logError(error: Error, errorInfo?: ErrorInfo): void {
        const errorLog: ErrorLog = {
            timestamp: Date.now(),
            message: error.message,
            stack: error.stack,
            componentStack: errorInfo?.componentStack || undefined,
            userAgent: navigator.userAgent,
            url: window.location.href
        };

        this.addToQueue(errorLog);
        this.logToConsole(errorLog);
        this.sendToBackend(errorLog);
    }

    logGeneralError(message: string, error?: unknown): void {
        const errorLog: ErrorLog = {
            timestamp: Date.now(),
            message,
            stack: error instanceof Error ? error.stack : String(error),
            userAgent: navigator.userAgent,
            url: window.location.href
        };

        this.addToQueue(errorLog);
        this.logToConsole(errorLog);
        this.sendToBackend(errorLog);
    }

    logInvokeError(command: string, params: unknown, error: unknown): void {
        const message = `Tauri invoke error: ${command}`;
        const errorLog: ErrorLog = {
            timestamp: Date.now(),
            message,
            stack: `Command: ${command}\nParams: ${JSON.stringify(params, null, 2)}\nError: ${error instanceof Error ? error.stack : String(error)}`,
            userAgent: navigator.userAgent,
            url: window.location.href
        };

        this.addToQueue(errorLog);
        this.logToConsole(errorLog);
        this.sendToBackend(errorLog);
    }

    private addToQueue(errorLog: ErrorLog): void {
        this.errorQueue.push(errorLog);

        if (this.errorQueue.length > this.maxQueueSize) {
            this.errorQueue.shift();
        }

        try {
            localStorage.setItem('vibe-error-logs', JSON.stringify(this.errorQueue));
        } catch (e) {
            console.error('Failed to store error logs:', e);
        }
    }

    private logToConsole(errorLog: ErrorLog): void {
        console.group(`🔴 VIBE Error - ${new Date(errorLog.timestamp).toLocaleTimeString()}`);
        console.error('Message:', errorLog.message);
        if (errorLog.stack) {
            console.error('Stack:', errorLog.stack);
        }
        if (errorLog.componentStack) {
            console.error('Component Stack:', errorLog.componentStack);
        }
        console.groupEnd();
    }

    private async sendToBackend(errorLog: ErrorLog): Promise<void> {
        try {
            await invoke('log_frontend_error', {
                timestamp: errorLog.timestamp,
                message: errorLog.message,
                stack: errorLog.stack || '',
                componentStack: errorLog.componentStack || '',
                userAgent: errorLog.userAgent,
                url: errorLog.url
            });
        } catch (e) {
            console.warn('Failed to send error to backend:', e);
        }
    }

    getErrorLogs(): ErrorLog[] {
        return [...this.errorQueue];
    }

    clearLogs(): void {
        this.errorQueue = [];
        try {
            localStorage.removeItem('vibe-error-logs');
        } catch (e) {
            console.error('Failed to clear error logs:', e);
        }
    }

    loadPersistedLogs(): void {
        try {
            const stored = localStorage.getItem('vibe-error-logs');
            if (stored) {
                this.errorQueue = JSON.parse(stored);
            }
        } catch (e) {
            console.error('Failed to load persisted error logs:', e);
        }
    }
}

const errorLogger = new ErrorLoggerService();
errorLogger.loadPersistedLogs();

window.addEventListener('error', (event) => {
    errorLogger.logGeneralError(event.message, event.error);
});

window.addEventListener('unhandledrejection', (event) => {
    errorLogger.logGeneralError('Unhandled Promise Rejection', event.reason);
});

export const logError = (error: Error, errorInfo?: ErrorInfo) =>
    errorLogger.logError(error, errorInfo);

export const logGeneralError = (message: string, error?: unknown) =>
    errorLogger.logGeneralError(message, error);

export const logInvokeError = (command: string, params: unknown, error: unknown) =>
    errorLogger.logInvokeError(command, params, error);

export const getErrorLogs = () => errorLogger.getErrorLogs();

export const clearErrorLogs = () => errorLogger.clearLogs();

export default errorLogger;
