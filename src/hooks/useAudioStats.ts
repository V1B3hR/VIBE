import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface AudioStats {
    isPlaying: boolean;
    masterLevel: number; // 0..1 (Peak L+R avg)
    cpuLoad: number;
    memoryUsage: number;
}

export function useAudioStats() {
    const [stats, setStats] = useState<AudioStats>({
        isPlaying: false,
        masterLevel: 0,
        cpuLoad: 0,
        memoryUsage: 0,
    });

    useEffect(() => {
        const fetchStats = async () => {
            try {
                const [playing, meters, cpu, mem] = await Promise.all([
                    invoke<boolean>('is_playing'),
                    invoke<{ peak_l_db: number; peak_r_db: number }>('get_master_meters'),
                    invoke<number>('get_cpu_load'),
                    invoke<number>('get_memory_usage'),
                ]);

                // Map dB to 0..1 linear
                const dbToLinear = (db: number) => Math.pow(10, db / 20);
                const linearL = dbToLinear(meters.peak_l_db);
                const linearR = dbToLinear(meters.peak_r_db);

                setStats({
                    isPlaying: playing,
                    masterLevel: (linearL + linearR) / 2,
                    cpuLoad: cpu,
                    memoryUsage: mem,
                });
            } catch (e) {
                // Backend might be offline
            }
        };

        const interval = setInterval(fetchStats, 50);
        return () => clearInterval(interval);
    }, []);

    return stats;
}
