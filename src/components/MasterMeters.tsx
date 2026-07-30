import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./MasterMeters.css";

interface MeterData {
    peak_l_db: number;
    peak_r_db: number;
    rms_l_db: number;
    rms_r_db: number;
}

export const MasterMeters = () => {
    const [meterData, setMeterData] = useState<MeterData>({
        peak_l_db: -96,
        peak_r_db: -96,
        rms_l_db: -96,
        rms_r_db: -96,
    });

    useEffect(() => {
        const interval = setInterval(async () => {
            try {
                const data = await invoke<MeterData>("get_master_meters");
                setMeterData(data);
            } catch (e) {
                // Backend might not be ready
            }
        }, 50); // 20 FPS update rate

        return () => clearInterval(interval);
    }, []);

    const dbToPercent = (db: number): number => {
        // -96dB to 0dB mapped to 0% to 100%
        return Math.max(0, Math.min(100, ((db + 96) / 96) * 100));
    };

    const getMeterColor = (db: number): string => {
        if (db > -3) return "#ff3333"; // Red (clipping)
        if (db > -6) return "#ffaa00"; // Orange (hot)
        if (db > -18) return "#00ff00"; // Green (good)
        return "#00aa00"; // Dark green (low)
    };

    const renderMeter = (label: string, peakDb: number, rmsDb: number) => {
        const peakPercent = dbToPercent(peakDb);
        const rmsPercent = dbToPercent(rmsDb);

        return (
            <div className="meter-channel">
                <div className="meter-label">{label}</div>
                <div className="meter-bar-container">
                    {/* RMS (background) */}
                    <div
                        className="meter-bar meter-rms"
                        style={{
                            width: `${rmsPercent}%`,
                            backgroundColor: getMeterColor(rmsDb),
                            opacity: 0.5,
                        }}
                    />
                    {/* Peak (foreground) */}
                    <div
                        className="meter-bar meter-peak"
                        style={{
                            width: `${peakPercent}%`,
                            backgroundColor: getMeterColor(peakDb),
                        }}
                    />
                    {/* Scale marks */}
                    <div className="meter-scale">
                        <div className="scale-mark" style={{ left: "25%" }}>-18</div>
                        <div className="scale-mark" style={{ left: "50%" }}>-12</div>
                        <div className="scale-mark" style={{ left: "75%" }}>-6</div>
                        <div className="scale-mark" style={{ left: "95%" }}>0</div>
                    </div>
                </div>
                <div className="meter-value">
                    <span className="peak-value">{(peakDb ?? -96).toFixed(1)} dB</span>
                    <span className="rms-value">RMS: {(rmsDb ?? -96).toFixed(1)} dB</span>
                </div>
            </div>
        );
    };

    return (
        <div className="master-meters" id="vibe-master-meters">
            <div className="meters-header">
                <span className="meters-title">MASTER METERS</span>
                <span className="meters-badge">GPU-OFFLOADED</span>
            </div>
            <div className="meters-content">
                {renderMeter("L", meterData.peak_l_db, meterData.rms_l_db)}
                {renderMeter("R", meterData.peak_r_db, meterData.rms_r_db)}
            </div>
        </div>
    );
};
