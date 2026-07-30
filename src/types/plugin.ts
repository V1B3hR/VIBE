export type PluginType = 'VST2' | 'VST3' | 'CLAP' | 'Native' | 'WASM';

export type PluginCategory =
    | 'Dynamics'
    | 'EQ'
    | 'Reverb'
    | 'Delay'
    | 'Distortion'
    | 'Modulation'
    | 'Instrument'
    | 'Utility'
    | 'MidiFX'
    | 'Other';

export interface PluginInfo {
    id: string;
    name: string;
    vendor: string;
    path: string;
    plugin_type: PluginType;
    category: PluginCategory;
    is_blacklisted: boolean;
    blacklist_reason?: string;
    thumbnail_path?: string;
    last_scanned: number;
    // Phase 1 extensions
    is_favorite: boolean;
    tags: string[];
    last_used?: number;
    custom_folder?: string;
    // Phase 3
    hidden: boolean;
    deprecated: boolean;
    duplicate_of?: string;
    cpu_usage_avg?: number;
    latency_samples?: number;
}

export type ChainRouting = 'Serial' | 'Parallel';

export interface PluginChain {
    id: string;
    name: string;
    plugins: string[]; // Plugin IDs
    routing: ChainRouting;
}

export interface ScanLogEntry {
    timestamp: number;
    path: string;
    status: 'Success' | 'Failed' | 'Skipped';
    error?: string;
}

export interface PluginDiagnostics {
    load_time_ms: number;
    memory_usage_bytes: number;
    error_count: number;
    last_crash?: number;
}

export interface PluginPreset {
    name: string;
    category: string;
    author?: string;
    path: string;
}
