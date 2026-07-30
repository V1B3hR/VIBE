export interface ParameterInfo {
    id: string;
    name: string;
    value: number;
    min_value: number;
    max_value: number;
    automation: { sample_pos: number; value: number }[];
}

export interface Clip {
    id: string;
    name: string;
    start_sample: number;
    duration_samples: number;
    peaks: number[][];
    offset_in_data: number;
    fade_in_len: number;
    fade_out_len: number;
    fade_in_type: string;
    fade_out_type: string;
    path?: string;
    warp_mode?: string;
    length_samples?: number; // Some parts of the code use length_samples
    color?: string;          // Per-clip color override (empty = inherit track color)
    gain?: number;           // Clip gain (linear)
}

export interface MidiNote {
    start_sample: number;
    length_samples: number;
    note: number;
    velocity: number;
    channel: number;
    probability: number;
    velocity_random: number;
    timing_random: number;
    pitch_bend?: number;
    pressure?: number;
    timbre?: number;
}

export interface Scale {
    root: number;
    type: string;
}

export interface ChordMarker {
    position_samples: number;
    chord_name: string;
    notes: number[];
}

export interface MidiClip {
    id: string;
    name: string;
    start_sample: number;
    length_samples: number;
    notes: MidiNote[];
    color: string;
    scale?: Scale;
    chord_markers?: ChordMarker[];
    groove_template?: string;
    pattern_id?: string;
    tuning_steps?: number;
    time_signature_num?: number;
    time_signature_den?: number;
    preview_notes?: [number, number, number][];
}

export interface EffectInfo {
    id: string;
    name: string;
    parameters: ParameterInfo[];
}

export interface Track {
    id: string;
    name: string;
    volume: ParameterInfo;
    is_muted: boolean;
    is_solo: boolean;
    is_armed?: boolean;
    is_disabled?: boolean;
    is_frozen?: boolean;
    automation_mode?: string;
    clips: Clip[];
    midi_clips: MidiClip[];
    color: string;
    effects?: EffectInfo[];
    track_type?: 'Audio' | 'MIDI' | 'Folder' | 'Bus';
    parent_id?: string | null;
    is_collapsed?: boolean;
    height?: number;
    take_count?: number;
    comp_mode_enabled?: boolean;
    comp_lanes?: Clip[][];
}

export interface Marker {
    id: string;
    label: string;
    pos: number;
    color: string;
}
