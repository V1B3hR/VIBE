import React, { useState, useEffect, useRef, type MouseEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { aiAssistant } from '../services/AiAssistService';
import { KropelkaPermissions } from './KropelkaPermissions';
import './Kropelka.css';

interface KropelkaProps {
    masterLevel: number; // 0..1
    isPlaying: boolean;
}

type KropelkaState = 'creative' | 'flow' | 'technical' | 'vibe_check' | 'idle';

interface InsightCard {
    category: 'Theory' | 'Mixing' | 'Safety' | 'Vibe' | 'Dynamics' | 'AI' | 'Groove';
    text: string;
    action?: string;
    choices?: string[];
    emotion?: string;
}

interface KropelkaBrainResponse {
    category: 'Theory' | 'Mixing' | 'Safety' | 'Vibe' | 'Dynamics' | 'AI' | 'Groove';
    text: string;
    action_type?: string;
    choices?: string[];
    state: string;
    emotion?: string;
}

export function Kropelka({ masterLevel, isPlaying }: KropelkaProps) {
    const [mood, setMood] = useState<KropelkaState>('idle');
    const [energy, setEnergy] = useState(0);
    const [insight, setInsight] = useState<InsightCard | null>(null);
    const [vibeCheckData, setVibeCheckData] = useState<{ rms: number; balance: number } | null>(null);
    const [showPermissions, setShowPermissions] = useState(false);
    const [isTransitioning, setIsTransitioning] = useState(false);

    const energyRef = useRef(0);
    const pollingRef = useRef<any>(null);
    const prevMoodRef = useRef<KropelkaState>('idle');

    // Smooth mood transition effect
    useEffect(() => {
        if (prevMoodRef.current !== mood) {
            setIsTransitioning(true);
            const timer = setTimeout(() => setIsTransitioning(false), 450);
            prevMoodRef.current = mood;
            return () => clearTimeout(timer);
        }
    }, [mood]);

    // 1. Audio Analysis Hook for Visuals
    useEffect(() => {
        // Smooth energy tracking
        if (masterLevel > energy) {
            setEnergy(masterLevel);
        } else {
            setEnergy((prev: number) => prev * 0.9 + masterLevel * 0.1);
        }

        // Basic Mood Determination derived from audio, overridden by Kropelka Logic later
        if (masterLevel > 0.95) {
            setMood('technical'); // Clipping alert
        } else if (isPlaying && masterLevel > 0.4) {
            // Only set to Flow if not already in a specific logic state (like Vibe Check)
            setMood((prev: KropelkaState) => (prev === 'vibe_check' ? 'vibe_check' : 'flow'));
        } else {
            setMood((prev: KropelkaState) => (prev === 'vibe_check' ? 'vibe_check' : 'creative'));
        }

        energyRef.current = masterLevel;
    }, [masterLevel, isPlaying]);

    // Zosia Samosia (Eco-Hygiene) Activity Pings
    useEffect(() => {
        if (isPlaying) {
            invoke('trigger_zosia_activity').catch(e => console.error("Zosia Activity Ping Failed", e));
        }
    }, [isPlaying]);

    // Zosia Samosia Audit Loop (Every 60s)
    useEffect(() => {
        const auditLoop = setInterval(() => {
            invoke<string>('trigger_zosia_audit').then(res => {
                if (res && res.includes("actions queued")) {
                    console.log("[ZosiaMind] Audit Complete: ", res);
                }
            }).catch(e => console.error("Zosia Audit Ping Failed", e));
        }, 60000);
        
        return () => clearInterval(auditLoop);
    }, []);

    // 2. Kropelka Brain Polling (The "Mind")
    useEffect(() => {
        const pollBrain = async () => {
            try {
                // Determine context based on basic state (In real app, getting focused window/plugin)
                let baseState = isPlaying ? "Mixing" : "Empty";
                if (masterLevel > 0.9) baseState = "Mastering";
                const pluginContext = aiAssistant.getContextData();

                let context = JSON.stringify({
                    projectState: baseState,
                    uiData: JSON.parse(pluginContext)
                });

                // 20% chance to check structure instead of general suggestion
                const shouldCheckStructure = Math.random() > 0.8;
                const command = shouldCheckStructure ? 'get_structure_analysis' : 'get_kropelka_suggestion';
                const args = shouldCheckStructure ? {} : { context };

                const res = await invoke<KropelkaBrainResponse>(command, args);

                if (res) {
                    setInsight({
                        category: res.category,
                        text: res.text,
                        action: res.action_type,
                        choices: res.choices,
                        emotion: res.emotion
                    });

                    // Mapping for ProducerMode -> Creative visual
                    const stateStr = res.state.toLowerCase();
                    const visualState = stateStr === 'producermode' ? 'creative' : stateStr;
                    setMood(visualState as KropelkaState);

                    setTimeout(() => setInsight(null), res.choices ? 12000 : 8000);
                }
            } catch (e) {
                console.error("Kropelka Brain Sleepy:", e);
            }
        };

        pollingRef.current = setInterval(pollBrain, 7000);
        return () => clearInterval(pollingRef.current);
    }, [isPlaying, masterLevel]);

    // 3. User Interaction
    const handleKropelkaClick = async () => {
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

    const [chatInput, setChatInput] = useState('');
    const [isChatting, setIsChatting] = useState(false);

    const handleChatSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!chatInput.trim()) return;

        try {
            setInsight({ category: 'AI', text: "myślę..." });
            const command = 'get_kropelka_suggestion';
            const contextPayload = JSON.stringify({
                projectState: chatInput,
                uiData: {}
            });
            const res = await invoke<KropelkaBrainResponse>(command, { context: contextPayload });
            
            if (res) {
                setInsight({
                    category: res.category,
                    text: res.text,
                    action: res.action_type,
                    choices: res.choices,
                    emotion: res.emotion
                });
                const stateStr = res.state.toLowerCase();
                setMood(stateStr === 'producermode' ? 'creative' : stateStr as KropelkaState);
                
                // If it generated a drum clip action natively, Zosia-samosia auto-triggers it
                if (res.action_type === 'GenerateDrumClip') {
                    // Automatically execute it! Zosia-samosia Mode
                    await invoke('apply_kropelka_fix', { action_type: res.action_type, action_data: null });
                }
            } else {
                setInsight({ category: 'Vibe', text: "Ciekawy pomył!" });
            }
        } catch (err) {
            console.error("Chat error", err);
        }
        setChatInput('');
        setIsChatting(false);
    };

    const handleAction = async (e: React.MouseEvent, action: string) => {
        e.stopPropagation();
        try {
            const res = await invoke<string>('apply_kropelka_fix', {
                action_type: action,
                action_data: null // We could pass data if we had it in setInsight
            });

            setInsight((prev: InsightCard | null) => prev ? { ...prev, text: res } : null);
            setTimeout(() => setInsight(null), 3500);
        } catch (err) {
            console.error("Kropelka Fix Failed:", err);
            setInsight((prev: InsightCard | null) => prev ? { ...prev, text: "Fix failed. I'll try harder next time! 🤕" } : null);
        }
    };

    const handleContextMenu = (e: React.MouseEvent) => {
        e.preventDefault();
        setShowPermissions(true);
    };

    return (
        <div className={`kropelka-container mode-${mood}${isTransitioning ? ' is-transitioning' : ''}`} onClick={handleKropelkaClick} onContextMenu={handleContextMenu}>

            <KropelkaPermissions isOpen={showPermissions} onClose={() => setShowPermissions(false)} />

            {/* Vibe Check Scanner Overlay */}
            {mood === 'vibe_check' && <div className="vibe-scanner-ring"></div>}

            {/* Core Avatar */}
            <div
                className="kropelka-core"
                style={{
                    transform: `scale(${1 + energy * 0.2})`,
                    filter: `drop-shadow(0 0 ${10 + energy * 20}px var(--glow-color))`
                }}
            >
                <div className="kropelka-face">
                    <div className={`eye left ${mood} ${insight?.emotion || ''}`}></div>
                    <div className={`eye right ${mood} ${insight?.emotion || ''}`}></div>
                    <div className={`mouth ${mood} ${insight?.emotion || ''}`}></div>
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
                            {insight.category === 'AI' && '🤖'}
                            {insight.category === 'Groove' && '🥁'}
                        </span>
                        <span className="card-title">{insight.category}</span>
                    </div>
                    <div className="card-body">
                        {insight.text}
                        {insight.choices ? (
                            <div className="card-actions" style={{ display: 'flex', gap: '8px', marginTop: '8px', flexWrap: 'wrap' }}>
                                {insight.choices.map((choice: string, i: number) => (
                                    <button
                                        key={i}
                                        className="fix-btn"
                                        style={choice.match(/Nah|No|Ignore/) ? { background: '#444' } : {}}
                                        onClick={(e: React.MouseEvent) => {
                                            if (choice.match(/Nah|No|Ignore/)) {
                                                if (insight.action) {
                                                    invoke('reject_kropelka_suggestion', { action_type: insight.action });
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
                                <button className="fix-btn" onClick={(e: React.MouseEvent) => handleAction(e, insight.action!)}>
                                    AKCEPTUJ SUGESTIE
                                </button>
                            )
                        )}
                    </div>
                </div>
            )}

            {/* Chat Overlay Toggle */}
            <div 
                className="chat-toggle-btn" 
                onClick={(e) => { e.stopPropagation(); setIsChatting(!isChatting); }}
                title="Porozmawiaj z Kropelką"
                style={{
                    position: 'absolute', bottom: '-40px', left: '50%', transform: 'translateX(-50%)',
                    background: 'rgba(20,20,30,0.8)', padding: '6px 12px', borderRadius: '15px',
                    color: '#0fb', fontSize: '12px', cursor: 'pointer', border: '1px solid #0fb', zIndex: 100
                }}
            >
                {isChatting ? 'Zamknij' : 'Rozmawiaj (Zosia Samosia)'}
            </div>

            {/* Chat Input Field */}
            {isChatting && (
                <form 
                    onSubmit={handleChatSubmit} 
                    onClick={(e) => e.stopPropagation()}
                    style={{
                        position: 'absolute', top: '-60px', left: '50%', transform: 'translateX(-50%)',
                        display: 'flex', gap: '10px', background: 'rgba(10,10,20,0.9)', 
                        padding: '10px', borderRadius: '8px', border: '1px solid #333', zIndex: 200, width: '250px'
                    }}
                >
                    <input 
                        type="text" 
                        value={chatInput} 
                        onChange={(e) => setChatInput(e.target.value)} 
                        placeholder="Napisz do Kropelki..."
                        autoFocus
                        style={{
                            background: '#000', color: '#fff', border: '1px solid #444', 
                            padding: '8px', borderRadius: '4px', flex: 1, outline: 'none'
                        }}
                    />
                    <button type="submit" style={{ background: '#0fb', color: '#000', border: 'none', padding: '0 12px', borderRadius: '4px', cursor: 'pointer', fontWeight: 'bold' }}>&rarr;</button>
                </form>
            )}

            {/* Vibe Stats */}
            {vibeCheckData && (
                <div className="kropelka-stats-holo">
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
