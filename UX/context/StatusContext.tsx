import React, { createContext, useContext, useState, ReactNode } from 'react';

interface StatusContextType {
    status: string | null;
    statusType: 'info' | 'error';
    isBusy: boolean;
    setStatus: (message: string, isBusy?: boolean) => void;
    setError: (message: string) => void;
    clearStatus: () => void;
}

const StatusContext = createContext<StatusContextType | undefined>(undefined);

export const StatusProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
    const [status, setStatusMessage] = useState<string | null>(null);
    const [statusType, setStatusType] = useState<'info' | 'error'>('info');
    const [isBusy, setIsBusy] = useState(false);

    const setStatus = (message: string, busy: boolean = true) => {
        setStatusMessage(message);
        setStatusType('info');
        setIsBusy(busy);
    };

    const setError = (message: string) => {
        setStatusMessage(message);
        setStatusType('error');
        setIsBusy(false);
        // Auto-clear error after 5 seconds
        setTimeout(() => {
            setStatusMessage(prev => prev === message ? null : prev);
        }, 5000);
    };

    const clearStatus = () => {
        setStatusMessage(null);
        setStatusType('info');
        setIsBusy(false);
    };

    return (
        <StatusContext.Provider value={{ status, statusType, isBusy, setStatus, setError, clearStatus }}>
            {children}
        </StatusContext.Provider>
    );
};

export const useStatus = () => {
    const context = useContext(StatusContext);
    if (!context) {
        throw new Error('useStatus must be used within a StatusProvider');
    }
    return context;
};
