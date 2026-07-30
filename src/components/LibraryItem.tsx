import React, { useRef } from "react";

export interface LibraryItemData {
    id: string; // UUID or Path
    name: string;
    path: string;
    duration_samples?: number; // Optional as plugins don't have it
    category?: string;
    waveform_peaks?: number[];
}

interface LibraryItemProps {
    item: LibraryItemData;
    type: 'clip' | 'plugin' | 'native';
    onPreviewStart: (path: string) => void;
    onPreviewStop: () => void;
    style?: React.CSSProperties;
}

export const LibraryItem: React.FC<LibraryItemProps> = ({ item, type, onPreviewStart, onPreviewStop, style }) => {
    const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
    const [isLatched, setIsLatched] = React.useState(false);

    const handleMouseEnter = () => {
        if (type !== 'clip') return;

        // Intelligent Preview: Wait 1.5s before triggering (per implementation plan)
        hoverTimer.current = setTimeout(() => {
            onPreviewStart(item.path);
        }, 1500);
    };

    const handleMouseLeave = () => {
        if (hoverTimer.current) {
            clearTimeout(hoverTimer.current);
            hoverTimer.current = null;
        }
        if (type === 'clip' && !isLatched) {
            onPreviewStop();
        }
    };

    const handleClick = () => {
        if (type === 'clip') {
            setIsLatched(!isLatched);
            // If latching just now, start preview immediately if not already playing
            if (!isLatched) {
                if (hoverTimer.current) {
                    clearTimeout(hoverTimer.current);
                    hoverTimer.current = null;
                }
                onPreviewStart(item.path);
            }
        }
    };

    const handleDragStart = (e: React.DragEvent) => {
        // Set Payload
        const payload = type === 'clip' ? item.id : item.path; // Plugins often use path as ID
        e.dataTransfer.setData(type === 'clip' ? "vibe/clip-id" : "vibe/plugin-id", payload);
        e.dataTransfer.effectAllowed = "copy";

        // Custom Ghost Image
        // We create a canvas to draw a customized representation
        const canvas = document.createElement("canvas");
        canvas.width = 200;
        canvas.height = 40;
        const ctx = canvas.getContext("2d");
        if (ctx) {
            // Background
            ctx.fillStyle = "#1a1a1a";
            ctx.fillRect(0, 0, 200, 40);
            ctx.strokeStyle = "#00d2fc"; // VIBE Cyan
            ctx.lineWidth = 2;
            ctx.strokeRect(1, 1, 198, 38);

            // Text
            ctx.fillStyle = "#ffffff";
            ctx.font = "12px Inter";
            ctx.fillText(item.name.substring(0, 25), 10, 24);

            // Waveform hint
            ctx.beginPath();
            ctx.moveTo(10, 35);
            ctx.lineTo(190, 35);
            ctx.strokeStyle = "#444";
            ctx.stroke();

            e.dataTransfer.setDragImage(canvas, 10, 20);
        }
    };

    return (
        <div
            className={`library-item ${type} ${isLatched ? 'latched' : ''}`}
            onMouseEnter={handleMouseEnter}
            onMouseLeave={handleMouseLeave}
            onClick={handleClick}
            draggable
            onDragStart={handleDragStart}
            style={style}
        >
            <div className="item-icon">
                {type === 'clip' ? '🎵' : type === 'plugin' ? '🔌' : '🎹'}
            </div>
            <div className="item-info">
                <span className="item-name">{item.name}</span>
                {item.duration_samples !== undefined && (
                    <span className="item-meta">{(item.duration_samples / 44100).toFixed(1)}s</span>
                )}
                {type === 'plugin' && <span className="item-meta">VST3</span>}
            </div>

            {/* Waveform Visualization */}
            {item.waveform_peaks && item.waveform_peaks.length > 0 && (
                <div className="item-waveform">
                    {item.waveform_peaks.map((val, idx) => {
                        // Frequency content logic:
                        // Lower indices usually correspond to lower frequencies in many analysis contexts,
                        // but since these are time-domain peaks, we use a simple heuristic:
                        // First 30% = Low, Next 40% = Mid, Last 30% = High
                        const progress = idx / item.waveform_peaks!.length;
                        let color = "var(--accent)"; // Default Cyan (High/Percussion)

                        if (progress < 0.3) {
                            color = "#8b5cf6"; // Deep Purple/Blue (Low End)
                        } else if (progress < 0.7) {
                            color = "#ffd700"; // Gold/Amber (Mid)
                        }

                        return (
                            <div
                                key={idx}
                                className="peak-bar"
                                style={{
                                    height: `${val * 100}%`,
                                    background: color
                                }}
                            />
                        );
                    })}
                </div>
            )}
        </div>
    );
};
