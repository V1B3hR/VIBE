import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Types matching Rust backend
export type KnobMode = 'Absolute' | 'Relative' | 'RelativeBinOffset' | 'Toggle';

export interface MidiBinding {
    id: string; // Uuid
    device_hash: number;
    channel: number;
    cc_number: number;
    resolution: 'SevenBit' | { FourteenBit: { msb_cc: number, lsb_cc: number } };
    targets: ParameterTarget[];
    mode: KnobMode;
    bidirectional: boolean;
}

export interface ParameterTarget {
    param_id: string;
    min: number;
    max: number;
    scale: number;
    invert: boolean;
}

interface MidiLearnContextType {
    isLearningMode: boolean;
    learningParamId: string | null;
    bindings: MidiBinding[];

    enterLearnMode: () => void;
    exitLearnMode: () => void;
    startLearningParameter: (paramId: string) => Promise<void>;
    removeBinding: (bindingId: string) => Promise<void>;
    getBindingForParam: (paramId: string) => MidiBinding | undefined;
}

const MidiLearnContext = createContext<MidiLearnContextType | undefined>(undefined);

export const MidiLearnProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [isLearningMode, setIsLearningMode] = useState(false);
    const [learningParamId, setLearningParamId] = useState<string | null>(null);
    const [bindings, setBindings] = useState<MidiBinding[]>([]);

    const refreshBindings = useCallback(async () => {
        try {
            // We need to implement get_midi_bindings command in Rust first! 
            // Fetch bindings from backend
            const data = await invoke<MidiBinding[]>('get_midi_bindings');
            setBindings(data);
            console.log("Bindings refreshed:", data);
        } catch (e) {
            console.error("Failed to fetch bindings", e);
        }
    }, []);

    const enterLearnMode = () => {
        setIsLearningMode(true);
        document.body.classList.add('midi-learn-active');
    };

    const exitLearnMode = () => {
        setIsLearningMode(false);
        setLearningParamId(null);
        document.body.classList.remove('midi-learn-active');
        // Notify backend to stop learning if needed, though usually implicit
    };

    const startLearningParameter = async (paramId: string) => {
        if (!isLearningMode) return;
        setLearningParamId(paramId);
        console.log(`[Neural Mapper] Listening for MIDI input for param: ${paramId}`);
        try {
            await invoke('start_midi_learn', { paramId });
        } catch (e) {
            console.error("Failed to start midi learn", e);
        }
    };

    const removeBinding = async (bindingId: string) => {
        try {
            await invoke('remove_midi_binding', { id: bindingId });
            await refreshBindings();
        } catch (e) {
            console.error("Failed to remove binding", e);
        }
    };

    const getBindingForParam = (paramId: string) => {
        return bindings.find(b => b.targets.some(t => t.param_id === paramId));
    };

    // Poll for bindings while in learn mode (Fallback for event system)
    useEffect(() => {
        let interval: ReturnType<typeof setInterval>;
        if (isLearningMode) {
            interval = setInterval(() => {
                refreshBindings();
            }, 200);
        }
        return () => {
            if (interval) clearInterval(interval);
        };
    }, [isLearningMode, refreshBindings]);

    // Check if currently learning param has been bound
    useEffect(() => {
        if (learningParamId) {
            const isBound = bindings.some(b => b.targets.some(t => t.param_id === learningParamId));
            if (isBound) {
                console.log(`[MidiLearn] Parameter ${learningParamId} bound successfully!`);
                setLearningParamId(null);
            }
        }
    }, [bindings, learningParamId]);

    return (
        <MidiLearnContext.Provider value={{
            isLearningMode,
            learningParamId,
            bindings,
            enterLearnMode,
            exitLearnMode,
            startLearningParameter,
            removeBinding,
            getBindingForParam
        }}>
            {children}
        </MidiLearnContext.Provider>
    );
};

export const useMidiLearn = () => {
    const context = useContext(MidiLearnContext);
    if (!context) {
        throw new Error('useMidiLearn must be used within a MidiLearnProvider');
    }
    return context;
};
