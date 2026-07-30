import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './GeneratorsPanel.css';

interface GeneratorsPanelProps {
    onGenerationComplete?: (clipInfo: any) => void;
}

export const GeneratorsPanel: React.FC<GeneratorsPanelProps> = ({ onGenerationComplete }) => {
    const [generatorType, setGeneratorType] = useState<'Drum' | 'Melody' | 'Chord'>('Drum');
    const [genre, setGenre] = useState('Techno (Berlin)');
    const [emotion, setEmotion] = useState('Dark'); // Fake emotional prompt for UI for now, translates to params
    const [isGenerating, setIsGenerating] = useState(false);

    // Draggable fake object representing the generated clip
    const [generatedClip, setGeneratedClip] = useState<any | null>(null);

    const handleGenerate = async () => {
        setIsGenerating(true);
        try {
            // Translate Emotion to 'Groove Archetype' or 'Motif Strength' etc.
            const archetype = emotion === 'Dark' ? 'Aggressive' : 'Laid-Back';
            const motif = emotion === 'Dark' ? 0.9 : 0.5;

            let clip: any = null;

            if (generatorType === 'Drum') {
                clip = await invoke('generate_drum_pattern', {
                    genre: genre,
                    bpm: 120.0,
                    density: emotion === 'Dark' ? 0.8 : 0.4,
                    swing: archetype === 'Laid-Back' ? 0.3 : 0.05,
                    humanization: 0.1,
                    grooveArchetype: archetype,
                    interplay: 0.5,
                    fillFrequency: 4,
                    microLayering: true
                });
            } else if (generatorType === 'Melody') {
                clip = await invoke('generate_melody_pattern', {
                    genre: genre,
                    bpm: 120.0,
                    density: 0.6,
                    rootNote: 36, // C2
                    scaleType: 'Minor',
                    instrumentType: 'Pluck',
                    motifStrength: motif,
                    syncopation: 0.6,
                    articulationStyle: 'Staccato',
                    contour: 'Up-Down',
                    breathing: 0.2
                });
            } else if (generatorType === 'Chord') {
                clip = await invoke('generate_chord_pattern', {
                    genre: genre,
                    bpm: 120.0,
                    rootNote: 36, // C2
                    scaleType: 'Minor',
                    rhythmStyle: 'Syncopated',
                    complexity: 0.7,
                    progressionPreset: '1-4-5-6',
                    voicingStyle: 'Open',
                    rhythmComplexity: 0.6,
                    substitutions: 0.2
                });
            }

            setGeneratedClip(clip);
            if (onGenerationComplete) {
                onGenerationComplete(clip);
            }
        } catch (e) {
            console.error("Generation failed:", e);
        } finally {
            setIsGenerating(false);
        }
    };

    const handleDragStart = (e: React.DragEvent) => {
        if (!generatedClip) return;
        e.dataTransfer.setData('application/json', JSON.stringify({
            type: 'midi_clip',
            clip: generatedClip,
            source: 'generator'
        }));
        e.dataTransfer.effectAllowed = 'copy';
    };

    return (
        <div className="generators-panel">
            <div className="gen-header">
                <h3>NeuralForest Groove Genetix</h3>
                <span className="badge">AI MODEL v0.8</span>
            </div>

            <div className="gen-controls">
                <div className="control-group">
                    <label>Target Class</label>
                    <select value={generatorType} onChange={(e) => setGeneratorType(e.target.value as any)}>
                        <option value="Drum">Drum Kit (Groove)</option>
                        <option value="Melody">Lead Melody (Motif)</option>
                        <option value="Chord">Chords (Progression)</option>
                    </select>
                </div>

                <div className="control-group">
                    <label>Genre Profile</label>
                    <select value={genre} onChange={(e) => setGenre(e.target.value)}>
                        <option value="Techno (Berlin)">Techno (Berlin)</option>
                        <option value="House (Deep)">House (Deep)</option>
                        <option value="Hip-Hop (Boom Bap)">Hip-Hop (Boom Bap)</option>
                        <option value="Cinematic">Cinematic Ambient</option>
                    </select>
                </div>

                <div className="control-group">
                    <label>Emotional Prompting</label>
                    <select value={emotion} onChange={(e) => setEmotion(e.target.value)}>
                        <option value="Dark">Dark & Aggressive</option>
                        <option value="Euphoric">Euphoric & Uplifting</option>
                        <option value="Melancholic">Melancholic (Sad)</option>
                        <option value="Groovy">Laid-back & Groovy (Dilla style)</option>
                    </select>
                </div>
                
                <p className="gen-hint">
                    Kropelka will use NeuralForest to synthesize a unique MIDI pattern modeled upon cognitive stylistic archetypes and MPC-60 micro-timing grids.
                </p>

                <button 
                    className={`btn-generate ${isGenerating ? 'loading' : ''}`}
                    onClick={handleGenerate}
                    disabled={isGenerating}
                >
                    {isGenerating ? 'Synthesizing...' : '⚡ GENERATE PATTERN'}
                </button>
            </div>

            {generatedClip && (
                <div className="gen-result">
                    <div className="result-header">Generation Complete</div>
                    <div 
                        className="draggable-pattern"
                        draggable
                        onDragStart={handleDragStart}
                        title="Drag this pattern to a MIDI Track"
                    >
                        <div className="pattern-icon">≡</div>
                        <div className="pattern-details">
                            <span className="pattern-name">{generatedClip.name || "Neural Pattern"}</span>
                            <span className="pattern-stats">{generatedClip.note_count} Notes • 4 Bars</span>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};
