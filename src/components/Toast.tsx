import { useState, useEffect, createContext, useContext, ReactNode } from 'react';
import './Toast.css';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
    id: string;
    message: string;
    type: ToastType;
    duration?: number;
}

interface ToastContextType {
    showToast: (message: string, type: ToastType, duration?: number) => void;
    showError: (message: string, duration?: number) => void;
    showSuccess: (message: string, duration?: number) => void;
    showWarning: (message: string, duration?: number) => void;
    showInfo: (message: string, duration?: number) => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

export const useToast = () => {
    const context = useContext(ToastContext);
    if (!context) {
        throw new Error('useToast must be used within ToastProvider');
    }
    return context;
};

interface ToastProviderProps {
    children: ReactNode;
}

export const ToastProvider = ({ children }: ToastProviderProps) => {
    const [toasts, setToasts] = useState<Toast[]>([]);

    const showToast = (message: string, type: ToastType, duration: number = 4000) => {
        const id = `toast-${Date.now()}-${Math.random()}`;
        const newToast: Toast = { id, message, type, duration };

        setToasts(prev => [...prev, newToast]);

        if (duration > 0) {
            setTimeout(() => {
                removeToast(id);
            }, duration);
        }
    };

    const removeToast = (id: string) => {
        setToasts(prev => prev.filter(toast => toast.id !== id));
    };

    const showError = (message: string, duration?: number) => showToast(message, 'error', duration);
    const showSuccess = (message: string, duration?: number) => showToast(message, 'success', duration);
    const showWarning = (message: string, duration?: number) => showToast(message, 'warning', duration);
    const showInfo = (message: string, duration?: number) => showToast(message, 'info', duration);

    return (
        <ToastContext.Provider value={{ showToast, showError, showSuccess, showWarning, showInfo }}>
            {children}
            <div className="toast-container">
                {toasts.map(toast => (
                    <ToastItem key={toast.id} toast={toast} onClose={() => removeToast(toast.id)} />
                ))}
            </div>
        </ToastContext.Provider>
    );
};

interface ToastItemProps {
    toast: Toast;
    onClose: () => void;
}

const ToastItem = ({ toast, onClose }: ToastItemProps) => {
    const [isExiting, setIsExiting] = useState(false);

    useEffect(() => {
        const timer = setTimeout(() => {
            setIsExiting(true);
        }, (toast.duration || 4000) - 300);

        return () => clearTimeout(timer);
    }, [toast.duration]);

    const getIcon = () => {
        switch (toast.type) {
            case 'success': return '✓';
            case 'error': return '✕';
            case 'warning': return '⚠';
            case 'info': return 'ℹ';
            default: return '';
        }
    };

    return (
        <div className={`toast toast-${toast.type} ${isExiting ? 'toast-exit' : 'toast-enter'}`}>
            <div className="toast-icon">{getIcon()}</div>
            <div className="toast-message">{toast.message}</div>
            <button className="toast-close" onClick={onClose} aria-label="Close">×</button>
        </div>
    );
};
