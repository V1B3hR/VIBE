import { Waveform } from "./Waveform";
import { MelSpectrogram } from "./MelSpectrogram";
import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect } from "react";
import "./SampleEditor.css";

interface Clip {
    id: string;
    name: string;
    peaks: number[][];
    trackIndex: number;
    start_sample: number;
    duration_samples: number;
}

interface SampleEditorProps {
    clip: Clip;
    onClose: () => void;
    docked?: boolean;
    playhead?: number;
}

export const SampleEditor = ({ clip, onClose, docked, playhead }: SampleEditorProps) => {
    const [viewMode, setViewMode] = useState<"waveform" | "spectrogram">("waveform");
    const [spectralData, setSpectralData] = useState<any>(null);
    const [loading, setLoading] = useState(false);
    const [gridPrecision, setGridPrecision] = useState("1/16");
    const [aiInsight, setAiInsight] = useState<string | null>(null);

    useEffect(() => {
        if (viewMode === "spectrogram" && !spectralData) {
            handleAnalyze();
        }
    }, [viewMode]);

    const handleAnalyze = async () => {
        setLoading(true);
        try {
            const result = await invoke("analyze_spectral", {
                trackIdx: clip.trackIndex,
                clipId: clip.id,
            });
            setSpectralData(result);
        } catch (e) {
            console.error("Spectral analysis failed", e);
        } finally {
            setLoading(false);
        }
    };

    const handleTrimToLoop = () => {
        // Mock function to trim sample to zero-crossings
        console.log("Trimming to perfect loop...");
        invoke('apply_kropelka_fix', { 
            action_type: 'TrimToLoop', 
            action_data: { clipId: clip.id, zeroCrossing: true } 
        });
    };

    const handleKropelkaAnalysis = async () => {
        setAiInsight("Kropelka analizuje pasmo i dynamikę sampla w kontekście całego utworu...");
        setTimeout(() => {
            setAiInsight("Analiza Zakończona: Próbka " + clip.name + " ma świetny atak, ale nieco 'płaskie' środkowe pasmo. Mogę wygenerować i nałożyć warstwę perkusyjną (shakery/haty) na siatkę " + gridPrecision + ", lub zregenerować melodię zachowując główny groove. Co wybierasz?");
        }, 1500);
    };

    const content = (
        <div className={docked ? "sample-editor-docked" : "sample-editor-modal glass pro-sampler"} onClick={(e) => e.stopPropagation()}>
            <div className="editor-header">
                <h2>VIBE PRO SAMPLER / {clip.name}</h2>
                <div className="view-selector btn-group">
                    <button className={`btn-tool ${viewMode === "waveform" ? "active" : ""}`} onClick={() => setViewMode("waveform")}>
                        Waveform
                    </button>
                    <button className={`btn-tool ${viewMode === "spectrogram" ? "active" : ""}`} onClick={() => setViewMode("spectrogram")}>
                        Spectrogram
                    </button>
                </div>
                {!docked && <button className="btn-close" onClick={onClose}>&times;</button>}
            </div>

            <div className="editor-main">
                <div className="editor-waveform-container">
                    {viewMode === "waveform" ? (
                        <Waveform peaks={clip.peaks} color="#0fb" />
                    ) : (
                        <MelSpectrogram
                            frames={spectralData?.frames || []}
                            width={1000}
                            height={400}
                            loading={loading}
                            playhead={playhead}
                            clipStart={clip.start_sample}
                            clipDuration={clip.duration_samples}
                        />
                    )}
                    <div className="sampler-playhead-overlay"></div>
                </div>

                {aiInsight && (
                    <div className="ai-sampler-insight">
                        <span className="insight-icon">🎶 KROPELKA:</span>
                        <p>{aiInsight}</p>
                        {aiInsight.includes("Co wybierasz?") && (
                            <div className="ai-insight-actions">
                                <button className="btn-ai-action">Zregeneruj Melodię</button>
                                <button className="btn-ai-action">Dogeneruj Haty ({gridPrecision})</button>
                                <button className="btn-ai-action">Dodaj Low-end / Sub</button>
                            </div>
                        )}
                    </div>
                )}
            </div>

            <div className="editor-tools-advanced">
                <div className="tool-section dsp-section">
                    <h3>DSP ENGINE</h3>
                    <div className="btn-grid">
                        <button className="btn-tool action-btn" onClick={handleTrimToLoop}>CROP TO PERFECT LOOP</button>
                        <button className="btn-tool">REVERSE & WARP</button>
                        <button className="btn-tool">NORMALIZE PEAKS</button>
                        <button className="btn-tool" onClick={handleKropelkaAnalysis} style={{color: '#0fb', borderColor: '#0fb'}}>KROPELKA ANALYSIS</button>
                    </div>
                </div>

                <div className="tool-section grid-section">
                    <h3>TIMING & GRID</h3>
                    <div className="grid-controls">
                        <label>Grid Resolution:</label>
                        <select className="dark-select" value={gridPrecision} onChange={(e) => setGridPrecision(e.target.value)}>
                            <option value="1/4">1/4</option>
                            <option value="1/8">1/8</option>
                            <option value="1/16">1/16</option>
                            <option value="1/32">1/32</option>
                            <option value="1/64">1/64</option>
                            <option value="1/128">1/128 (Micro)</option>
                        </select>
                        <button className="btn-tool">QUANTIZE</button>
                    </div>
                </div>

                <div className="tool-section envelope-section">
                    <h3>ADSR & FADES</h3>
                    <div className="fade-controls horizontal-fades">
                        <div className="control">
                            <label>Attack</label>
                            <input type="range" min="0" max="1000" defaultValue="10" />
                        </div>
                        <div className="control">
                            <label>Release</label>
                            <input type="range" min="0" max="1000" defaultValue="100" />
                        </div>
                    </div>
                </div>
            </div>
            
            <div className="resizer-handle"></div>
        </div>
    );

    if (docked) {
        return content;
    }

    return (
        <div className="sample-editor-overlay" onClick={onClose}>
            {content}
        </div>
    );
};
