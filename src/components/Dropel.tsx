import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './Dropel.css';

interface DropelProps {
    masterLevel: number; // 0..1
    isPlaying: boolean;
}

type DropelState = 'creative' | 'flow' | 'technical' | 'vibe_check' | 'idle';

interface InsightCard {
    category: 'Theory' | 'Mixing' | 'Safety' | 'Vibe' | 'Dynamics';
    text: string;
    action?: string;
    choices?: string[];
}

export function Dropel({ masterLevel, isPlaying }: DropelProps) {
    const [mood, setMood] = useState<DropelState>('idle');
    const [energy, setEnergy] = useState(0);
    const [insight, setInsight] = useState<InsightCard | null>(null);
    const [vibeCheckData, setVibeCheckData] = useState<{ rms: number; balance: number } | null>(null);

    const energyRef = useRef(0);
    const pollingRef = useRef<any>(null);

    // 1. Audio Analysis Hook for Visuals
    useEffect(() => {
        // Smooth energy tracking
        if (masterLevel > energy) {
            setEnergy(masterLevel);
        } else {
            setEnergy(prev => prev * 0.9 + masterLevel * 0.1);
        }

        // Basic Mood Determination derived from audio, overridden by Dropel Logic later
        if (masterLevel > 0.95) {
            setMood('technical'); // Clipping alert
        } else if (isPlaying && masterLevel > 0.4) {
            // Only set to Flow if not already in a specific logic state (like Vibe Check)
            setMood(prev => (prev === 'vibe_check' ? 'vibe_check' : 'flow'));
        } else {
            setMood(prev => (prev === 'vibe_check' ? 'vibe_check' : 'creative'));
        }

        energyRef.current = masterLevel;
    }, [masterLevel, isPlaying]);

    // 2. Dropel Brain Polling (The "Mind")
    useEffect(() => {
        const pollBrain = async () => {
            try {
                // Determine context based on basic state (In real app, getting focused window/plugin)
                let context = isPlaying ? "Mixing" : "Empty";
                if (masterLevel > 0.9) context = "Mastering";

                // 20% chance to check structure instead of general suggestion
                const shouldCheckStructure = Math.random() > 0.8;
                const command = shouldCheckStructure ? 'get_structure_analysis' : 'get_dropel_suggestion';
                const args = shouldCheckStructure ? {} : { context };

                const res = await invoke<any>(command, args);

                if (res) {
                    setInsight({
                        category: res.category,
                        text: res.text,
                        action: res.action_type,
                        choices: res.choices
                    });

                    // Mapping for ProducerMode -> Creative visual
                    const stateStr = res.state.toLowerCase();
                    const visualState = stateStr === 'producermode' ? 'creative' : stateStr;
                    setMood(visualState as DropelState);

                    setTimeout(() => setInsight(null), res.choices ? 12000 : 8000);
                }
            } catch (e) {
                console.error("Dropel Brain Sleepy:", e);
            }
        };

        pollingRef.current = setInterval(pollBrain, 7000);
        return () => clearInterval(pollingRef.current);
    }, [isPlaying, masterLevel]);

    // 3. User Interaction
    const handleDropelClick = async () => {
        // Trigger Manual Vibe Check + Key Detection
        setMood('vibe_check');
        setInsight({ category: 'Vibe', text: "Analyzing project soul... 📶" });

        await invoke('trigger_vibe_check');
        const keyResult = await invoke<[string, string] | null>('detect_project_key');

        setTimeout(() => {
            setVibeCheckData({ rms: masterLevel, balance: 0.6 });
            if (keyResult) {
                setInsight({ category: 'Theory', text: `Vibe Check Complete. Detected: ${keyResult[0]} (${keyResult[1]})` });
            } else {
                setInsight({ category: 'Vibe', text: "Vibe Check Complete. Solid dynamics!" });
            }

            setTimeout(() => {
                setVibeCheckData(null);
                setInsight(null);
                setMood('idle');
            }, 5000);
        }, 1200);
    };

    const handleAction = async (e: React.MouseEvent, action: string) => {
        e.stopPropagation();
        try {
            const res = await invoke<string>('apply_dropel_fix', {
                actionType: action,
                actionData: null // We could pass data if we had it in setInsight
            });

            setInsight(prev => prev ? { ...prev, text: res } : null);
            setTimeout(() => setInsight(null), 3500);
        } catch (err) {
            console.error("Dropel Fix Failed:", err);
            setInsight(prev => prev ? { ...prev, text: "Fix failed. I'll try harder next time! 🤕" } : null);
        }
    };

    return (
        <div className={`dropel-container mode-${mood}`} onClick={handleDropelClick}>

            {/* Vibe Check Scanner Overlay */}
            {mood === 'vibe_check' && <div className="vibe-scanner-ring"></div>}

            {/* Core Avatar */}
            <div
                className="dropel-core"
                style={{
                    transform: `scale(${1 + energy * 0.2})`,
                    filter: `drop-shadow(0 0 ${10 + energy * 20}px var(--glow-color))`
                }}
            >
                <div className="dropel-face">
                    <div className={`eye left ${mood}`}></div>
                    <div className={`eye right ${mood}`}></div>
                    <div className={`mouth ${mood}`}></div>
                </div>
            </div>

            {/* Insight Card */}
            {insight && (
                <div className={`insight-card cat-${insight.category.toLowerCase()}`}>
                    <div className="card-header">
                        <span className="card-icon">
                            {insight.category === 'Theory' && '🎵'}
                            {insight.category === 'Mixing' && '🎚️'}
                            {insight.category === 'Safety' && '🚨'}
                            {insight.category === 'Vibe' && '✨'}
                        </span>
                        <span className="card-title">{insight.category}</span>
                    </div>
                    <div className="card-body">
                        {insight.text}
                        {insight.choices ? (
                            <div className="card-actions" style={{ display: 'flex', gap: '8px', marginTop: '8px', flexWrap: 'wrap' }}>
                                {insight.choices.map((choice, i) => (
                                    <button
                                        key={i}
                                        className="fix-btn"
                                        style={choice.match(/Nah|No|Ignore/) ? { background: '#444' } : {}}
                                        onClick={(e) => {
                                            if (choice.match(/Nah|No|Ignore/)) {
                                                if (insight.action) {
                                                    invoke('reject_dropel_suggestion', { actionType: insight.action });
                                                }
                                                setInsight(null);
                                            } else if (i === 0 && insight.action) {
                                                handleAction(e, insight.action);
                                            }
                                        }}
                                    >
                                        {choice}
                                    </button>
                                ))}
                            </div>
                        ) : (
                            insight.action && (
                                <button className="fix-btn" onClick={(e) => handleAction(e, insight.action!)}>
                                    FIX IT
                                </button>
                            )
                        )}
                    </div>
                </div>
            )}

            {/* Vibe Stats */}
            {vibeCheckData && (
                <div className="vibe-stats-holo">
                    <div className="stat-row">
                        <span>RMS</span>
                        <div className="bar"><div style={{ width: `${vibeCheckData.rms * 100}%` }}></div></div>
                    </div>
                    <div className="stat-row">
                        <span>BAL</span>
                        <div className="bar"><div style={{ width: `${vibeCheckData.balance * 100}%` }}></div></div>
                    </div>
                </div>
            )}
        </div>
    );
}
