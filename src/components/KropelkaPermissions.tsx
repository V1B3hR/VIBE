import React, { useState } from 'react';
import './KropelkaPermissions.css';

interface KropelkaPermissionsProps {
    isOpen: boolean;
    onClose: () => void;
}

export const KropelkaPermissions: React.FC<KropelkaPermissionsProps> = ({ isOpen, onClose }) => {
    const [autonomyLevel, setAutonomyLevel] = useState<'low' | 'medium' | 'high'>('medium');
    
    // Focus areas
    const [assistArrangement, setAssistArrangement] = useState(true);
    const [assistPlugins, setAssistPlugins] = useState(true);
    const [assistHygiene, setAssistHygiene] = useState(true);
    const [assistComposition, setAssistComposition] = useState(false); // Music theory/Generators

    if (!isOpen) return null;

    return (
        <div className="kp-overlay">
            <div className="kp-modal">
                <div className="kp-header">
                    <div className="kp-title">
                        <span className="kp-avatar">⚙️</span>
                        <h3>Kropelka & Zosia Autonomy</h3>
                    </div>
                    <button className="kp-close" onClick={onClose}>×</button>
                </div>

                <div className="kp-body">
                    <div className="kp-section">
                        <h4>Autonomy Level (Interference)</h4>
                        <div className="kp-level-selector">
                            <button 
                                className={`kp-level-btn ${autonomyLevel === 'low' ? 'active low' : ''}`}
                                onClick={() => setAutonomyLevel('low')}
                            >
                                <span className="icon">👁️</span>
                                <div>
                                    <h5>Low (Observer)</h5>
                                    <p>Suggestions only. Kropelka never touches your knobs.</p>
                                </div>
                            </button>
                            <button 
                                className={`kp-level-btn ${autonomyLevel === 'medium' ? 'active medium' : ''}`}
                                onClick={() => setAutonomyLevel('medium')}
                            >
                                <span className="icon">🤝</span>
                                <div>
                                    <h5>Medium (Co-Pilot)</h5>
                                    <p>Auto-hygiene (Zosia) enabled. Needs approval for mix changes.</p>
                                </div>
                            </button>
                            <button 
                                className={`kp-level-btn ${autonomyLevel === 'high' ? 'active high' : ''}`}
                                onClick={() => setAutonomyLevel('high')}
                            >
                                <span className="icon">⚡</span>
                                <div>
                                    <h5>High (Zosia Samosia)</h5>
                                    <p>Full autonomy. Auto-cures "Mud", draws EQ curves silently.</p>
                                </div>
                            </button>
                        </div>
                    </div>

                    <div className="kp-section">
                        <h4>Focus Areas (Where should she help?)</h4>
                        <div className="kp-toggles-grid">
                            <label className={`kp-toggle-card ${assistArrangement ? 'active' : ''}`}>
                                <input type="checkbox" checked={assistArrangement} onChange={(e) => setAssistArrangement(e.target.checked)} />
                                <span className="title">🏗️ Arrangement</span>
                                <span className="desc">Macro-timeline pacing & transitions</span>
                            </label>

                            <label className={`kp-toggle-card ${assistPlugins ? 'active' : ''}`}>
                                <input type="checkbox" checked={assistPlugins} onChange={(e) => setAssistPlugins(e.target.checked)} />
                                <span className="title">🎛️ Plugins & Mix</span>
                                <span className="desc">Smart EQ, Compressors, Reverb</span>
                            </label>

                            <label className={`kp-toggle-card ${assistComposition ? 'active' : ''}`}>
                                <input type="checkbox" checked={assistComposition} onChange={(e) => setAssistComposition(e.target.checked)} />
                                <span className="title">🎹 Composition</span>
                                <span className="desc">Music Theory & MIDI Generations</span>
                            </label>

                            <label className={`kp-toggle-card ${assistHygiene ? 'active' : ''}`}>
                                <input type="checkbox" checked={assistHygiene} onChange={(e) => setAssistHygiene(e.target.checked)} />
                                <span className="title">🧹 Eco-Hygiene</span>
                                <span className="desc">Zosia compresses backups & cleans orphans</span>
                            </label>
                        </div>
                    </div>

                    <div className="kp-footer">
                        <div className="kp-status">
                            {autonomyLevel === 'high' ? "Zosia feels trusted. 🚀" : "Kropelka remains calm. 🌊"}
                        </div>
                        <button className="kp-save-btn" onClick={onClose}>Apply & Close</button>
                    </div>
                </div>
            </div>
        </div>
    );
};
