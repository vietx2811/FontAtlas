import React from 'react';
import { Loader2 } from 'lucide-react';
import { useStatus } from '../context/StatusContext';

export const StatusBar: React.FC = () => {
    const { status, statusType, isBusy } = useStatus();

    if (!status) return null;

    const baseStyles = "fixed bottom-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 px-4 py-2 backdrop-blur-md rounded-full shadow-lg border animate-in fade-in slide-in-from-bottom-2";
    const typeStyles = statusType === 'error'
        ? "bg-red-900/90 text-red-50 border-red-500/30"
        : "bg-stone-900/90 text-white border-white/10";

    return (
        <div className={`${baseStyles} ${typeStyles}`}>
            {isBusy && <Loader2 size={14} className="animate-spin text-stone-300" />}
            <span className="text-xs font-medium tracking-wide">{status}</span>
        </div>
    );
};
