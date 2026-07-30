import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './KropelkaPanel.css';

interface KropelkaPanelProps {
    isOpen: boolean;
    onClose: () => void;
    onAddClipToTimeline: (clipData: any) => void;
}

export const KropelkaPanel: React.FC<KropelkaPanelProps> = ({ isOpen, onClose, onAddClipToTimeline }) => {
    const [activeTab, setActiveTab] = useState<'Drums' | 'Melody' | 'Chords' | 'Structure'>('Drums');
    const [genre, setGenre] = useState('Techno');
    const [density, setDensity] = useState(0.8);
    const [isGenerating, setIsGenerating] = useState(false);

    // Global Macro
    const [humanization, setHumanization] = useState(0.5);
    const [lengthProfile, setLengthProfile] = useState('Radio Edit');

    // Drum specifics
    const [swing, setSwing] = useState(0.0);
    const [grooveArchetype, setGrooveArchetype] = useState('Straight');
    const [interplay, setInterplay] = useState(0.5);
    const [fillFrequency, setFillFrequency] = useState(8);
    const [microLayering, setMicroLayering] = useState(false);

    // Melody specifics
    const [rootNote, setRootNote] = useState(60); // C4
    const [scaleType, setScaleType] = useState('Minor');
    const [instrumentType, setInstrumentType] = useState('Synth');
    const [motifStrength, setMotifStrength] = useState(0.7);
    const [syncopation, setSyncopation] = useState(0.3);
    const [articulationStyle, setArticulationStyle] = useState('Legato');
    const [contour, setContour] = useState('Random');
    const [breathing, setBreathing] = useState(0.2);

    // Chord specifics
    const [progressionPreset, setProgressionPreset] = useState('Tension Arc');
    const [voicingStyle, setVoicingStyle] = useState('Piano Wide');
    const [rhythmComplexity, setRhythmComplexity] = useState(0.2);
    const [substitutions, setSubstitutions] = useState(0.1);

    const handleGenerate = async () => {
        setIsGenerating(true);
        try {
            let clipData = null;

            if (activeTab === 'Drums') {
                clipData = await invoke('generate_drum_pattern', {
                    genre,
                    bpm: 120.0, // Would be fetched from global state in full app
                    density,
                    swing,
                    humanization,
                    grooveArchetype,
                    interplay,
                    fillFrequency,
                    microLayering
                });
            } else if (activeTab === 'Melody') {
                clipData = await invoke('generate_melody_pattern', {
                    genre,
                    bpm: 120.0,
                    density,
                    rootNote,
                    scaleType,
                    instrumentType,
                    motifStrength,
                    syncopation,
                    articulationStyle,
                    contour,
                    breathing
                });
            } else if (activeTab === 'Chords') {
                clipData = await invoke('generate_chord_pattern', {
                    genre,
                    bpm: 120.0,
                    rootNote,
                    scaleType,
                    rhythmStyle: 'Sustained', // Deprecated in backend struct but kept in UI arg
                    complexity: density,
                    progressionPreset,
                    voicingStyle,
                    rhythmComplexity,
                    substitutions
                });
            } else if (activeTab === 'Structure') {
                const arr = await invoke('generate_intelligent_arrangement', {
                    genre,
                    bpm: 120.0,
                    rootNote,
                    scaleType,
                    lengthProfile
                });
                console.log("Generated Arrangement: ", arr);
                // Currently just logging. Timeline handling will be extended.
                if (onAddClipToTimeline) onAddClipToTimeline(arr);
                return;
            }

            if (clipData) {
                // Find active track (assuming track 0 for now as fallback)
                // Try to get track index from global state or find the first MIDI track
                const tracks = await invoke<any[]>('get_tracks').catch(() => []);
                let targetTrackIdx = tracks.findIndex((t: any) => t.track_type === 'MIDI' || t.track_type === 'Instrument');
                if (targetTrackIdx === -1) targetTrackIdx = 0; // Fallback to first track

                // Add to timeline
                await invoke('add_midi_clip', {
                    trackIdx: targetTrackIdx,
                    clip: clipData
                });

                if (onAddClipToTimeline) onAddClipToTimeline(clipData);
            }

        } catch (e) {
            console.error("AI Generation Error:", e);
        } finally {
            setIsGenerating(false);
        }
    };

    if (!isOpen) return null;

    return (
        <div className="kropelka-panel">
            <div className="kropelka-header">
                <h3>💧 Kropelka Co-Producer</h3>
                <button onClick={onClose} className="close-btn">×</button>
            </div>

            <div className="kropelka-tabs">
                <button className={activeTab === 'Drums' ? 'active' : ''} onClick={() => setActiveTab('Drums')}>🥁 Drums</button>
                <button className={activeTab === 'Chords' ? 'active' : ''} onClick={() => setActiveTab('Chords')}>🎹 Chords</button>
                <button className={activeTab === 'Melody' ? 'active' : ''} onClick={() => setActiveTab('Melody')}>🎼 Melody</button>
                <button className={activeTab === 'Structure' ? 'active' : ''} onClick={() => setActiveTab('Structure')}>🏗️ Arrangement</button>
            </div>

            <div className="kropelka-body">
                <div className="kropelka-row">
                    <div className="kropelka-control-group">
                        <label>Genre / Vibe</label>
                        <select value={genre} onChange={e => setGenre(e.target.value)}>
                            <option value="Techno">Techno</option>
                            <option value="House">House</option>
                            <option value="Hip-Hop">Hip-Hop / Boom Bap</option>
                            <option value="Trap">Trap</option>
                            <option value="Lo-Fi">Lo-Fi Soul</option>
                            <option value="Cinematic">Cinematic Scoring</option>
                        </select>
                    </div>

                    <div className="kropelka-control-group">
                        <label>Density: {density.toFixed(2)}</label>
                        <input type="range" min="0" max="1" step="0.05" value={density} onChange={e => setDensity(parseFloat(e.target.value))} />
                    </div>
                </div>

                <div className="kropelka-control-group">
                    <label>Humanization (Robot ↔ Human): {humanization.toFixed(2)}</label>
                    <input type="range" min="0" max="1" step="0.05" value={humanization} onChange={e => setHumanization(parseFloat(e.target.value))} />
                </div>

                {/* DRUMS SPECIFIC */}
                {activeTab === 'Drums' && (
                    <>
                        <div className="kropelka-row">
                            <div className="kropelka-control-group">
                                <label>Groove Archetype</label>
                                <select value={grooveArchetype} onChange={e => setGrooveArchetype(e.target.value)}>
                                    <option value="Straight">Straight (4/4)</option>
                                    <option value="Funky">Funky (Syncopated)</option>
                                    <option value="Half-Time">Half-Time</option>
                                    <option value="Broken">Broken / UKG</option>
                                </select>
                            </div>
                            <div className="kropelka-control-group">
                                <label>Fills (Every N Bars)</label>
                                <select value={fillFrequency} onChange={e => setFillFrequency(parseInt(e.target.value))}>
                                    <option value="0">Off</option>
                                    <option value="4">4 Bars</option>
                                    <option value="8">8 Bars</option>
                                    <option value="16">16 Bars</option>
                                </select>
                            </div>
                        </div>
                        <div className="kropelka-row">
                            <div className="kropelka-control-group">
                                <label>Interplay (Call & Response): {interplay.toFixed(2)}</label>
                                <input type="range" min="0" max="1" step="0.05" value={interplay} onChange={e => setInterplay(parseFloat(e.target.value))} />
                            </div>
                            <div className="kropelka-control-group" style={{ flexDirection: 'row', alignItems: 'center', marginTop: '12px' }}>
                                <input type="checkbox" checked={microLayering} onChange={e => setMicroLayering(e.target.checked)} />
                                <label style={{ marginLeft: '6px', marginTop: '0' }}>Sub-Kick Layering</label>
                            </div>
                        </div>
                    </>
                )}

                {/* MELODY SPECIFIC */}
                {activeTab === 'Melody' && (
                    <>
                        <div className="kropelka-row">
                            <div className="kropelka-control-group">
                                <label>Scale</label>
                                <select value={scaleType} onChange={e => setScaleType(e.target.value)}>
                                    <option value="Major">Major</option>
                                    <option value="Minor">Natural Minor</option>
                                    <option value="Pentatonic">Pentatonic</option>
                                    <option value="Hirajoshi">Hirajoshi (Cinematic)</option>
                                </select>
                            </div>
                            <div className="kropelka-control-group">
                                <label>Instrument Behavior</label>
                                <select value={instrumentType} onChange={e => setInstrumentType(e.target.value)}>
                                    <option value="Synth">Modern Synth Lead</option>
                                    <option value="Piano">Classic Piano</option>
                                    <option value="Pluck">Short Pluck/Arp</option>
                                    <option value="Pad">Slow Pad</option>
                                </select>
                            </div>
                        </div>
                        <div className="kropelka-row">
                            <div className="kropelka-control-group">
                                <label>Contour</label>
                                <select value={contour} onChange={e => setContour(e.target.value)}>
                                    <option value="Random">Free / Random</option>
                                    <option value="Ascending">Ascending Arc</option>
                                    <option value="Arch">Tension Arch</option>
                                </select>
                            </div>
                            <div className="kropelka-control-group">
                                <label>Articulation</label>
                                <select value={articulationStyle} onChange={e => setArticulationStyle(e.target.value)}>
                                    <option value="Legato">Legato (Glide)</option>
                                    <option value="Staccato">Staccato (Short)</option>
                                </select>
                            </div>
                        </div>
                        <div className="kropelka-control-group">
                            <label>Motif Strength (Repetition): {motifStrength.toFixed(2)}</label>
                            <input type="range" min="0" max="1" step="0.05" value={motifStrength} onChange={e => setMotifStrength(parseFloat(e.target.value))} />
                        </div>
                        <div className="kropelka-control-group">
                            <label>Syncopation: {syncopation.toFixed(2)}</label>
                            <input type="range" min="0" max="1" step="0.05" value={syncopation} onChange={e => setSyncopation(parseFloat(e.target.value))} />
                        </div>
                    </>
                )}

                {/* CHORDS SPECIFIC */}
                {activeTab === 'Chords' && (
                    <>
                        <div className="kropelka-row">
                            <div className="kropelka-control-group">
                                <label>Progression Preset</label>
                                <select value={progressionPreset} onChange={e => setProgressionPreset(e.target.value)}>
                                    <option value="Tension Arc">Tension Arc</option>
                                    <option value="Story Mode">Story Mode Context</option>
                                    <option value="Jazzy Loop">Neo-Soul / Jazz</option>
                                    <option value="EDM">EDM Minor</option>
                                    <option value="Pop">Standard Pop</option>
                                </select>
                            </div>
                            <div className="kropelka-control-group">
                                <label>Voicing Style</label>
                                <select value={voicingStyle} onChange={e => setVoicingStyle(e.target.value)}>
                                    <option value="Standard">Standard</option>
                                    <option value="Piano Wide">Piano Wide (Drop 2)</option>
                                    <option value="Pad Cluster">Pad Cluster (Close)</option>
                                </select>
                            </div>
                        </div>
                        <div className="kropelka-control-group">
                            <label>Rhythm Config (Pad ↔ Syncopated): {rhythmComplexity.toFixed(2)}</label>
                            <input type="range" min="0" max="1" step="0.05" value={rhythmComplexity} onChange={e => setRhythmComplexity(parseFloat(e.target.value))} />
                        </div>
                        <div className="kropelka-control-group">
                            <label>Experimental Substitutions (Safe ↔ Jazzy): {substitutions.toFixed(2)}</label>
                            <input type="range" min="0" max="1" step="0.05" value={substitutions} onChange={e => setSubstitutions(parseFloat(e.target.value))} />
                        </div>
                    </>
                )}

                {/* STRUCTURE SPECIFIC */}
                {activeTab === 'Structure' && (
                    <div className="kropelka-control-group">
                        <label>Length Profile</label>
                        <select value={lengthProfile} onChange={e => setLengthProfile(e.target.value)}>
                            <option value="Radio Edit">Radio Edit (3-4 mins)</option>
                            <option value="Extended Mix">Extended Mix (Club focus)</option>
                            <option value="Short Loop">Short Loop (1 min)</option>
                        </select>
                        <div className="offline-badge" style={{ marginTop: '12px', fontSize: '10px' }}>
                            Orchestrator will generate arrangement regions and distribute sub-tasks to Drum, Melody, and Chord Engines.
                        </div>
                    </div>
                )}

                <button
                    className={`kropelka-generate-btn ${isGenerating ? 'generating' : ''}`}
                    onClick={handleGenerate}
                    disabled={isGenerating}
                >
                    {isGenerating ? '🧠 Generating Offline...' : '✨ Generate Intelligent Clip'}
                </button>
            </div>
        </div>
    );
};
