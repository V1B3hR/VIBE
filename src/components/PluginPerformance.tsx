import React, { useMemo } from 'react';
import { PluginInfo } from '../types/plugin';
import './PluginPerformance.css';

interface PluginPerformanceProps {
    plugins: PluginInfo[];
}

export const PluginPerformance: React.FC<PluginPerformanceProps> = ({ plugins }) => {
    const topCpuPlugins = useMemo(() => {
        return [...plugins]
            .filter(p => p.cpu_usage_avg !== undefined)
            .sort((a, b) => (b.cpu_usage_avg || 0) - (a.cpu_usage_avg || 0))
            .slice(0, 5);
    }, [plugins]);

    const topLatencyPlugins = useMemo(() => {
        return [...plugins]
            .filter(p => p.latency_samples !== undefined && p.latency_samples > 0)
            .sort((a, b) => (b.latency_samples || 0) - (a.latency_samples || 0))
            .slice(0, 5);
    }, [plugins]);

    return (
        <div className="plugin-performance">
            <div className="perf-section">
                <h4>🔥 Top CPU Usage</h4>
                <div className="perf-list">
                    {topCpuPlugins.length > 0 ? topCpuPlugins.map(p => (
                        <div key={p.id} className="perf-item">
                            <span className="perf-name">{p.name}</span>
                            <span className="perf-value cpu">{(p.cpu_usage_avg || 0).toFixed(1)}%</span>
                        </div>
                    )) : <div className="perf-empty">No performance data.</div>}
                </div>
            </div>

            <div className="perf-section">
                <h4>⏳ Top Latency</h4>
                <div className="perf-list">
                    {topLatencyPlugins.length > 0 ? topLatencyPlugins.map(p => (
                        <div key={p.id} className="perf-item">
                            <span className="perf-name">{p.name}</span>
                            <span className="perf-value latency">{p.latency_samples} spls</span>
                        </div>
                    )) : <div className="perf-empty">No latency data.</div>}
                </div>
            </div>
        </div>
    );
};
