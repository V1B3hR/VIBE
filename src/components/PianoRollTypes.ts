// ============================================================================
// TYPE DEFINITIONS & CONSTANTS FOR PIANO ROLL
// ============================================================================

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

export interface ChordMarker {
    position_samples: number;
    chord_name: string;
    notes: number[];
}

export interface Scale {
    root: number;
    type: string;
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
}

export interface MidiClipInfo {
    id: string;
    name: string;
    start_sample: number;
    length_samples: number;
    color: string;
    pattern_id?: string;
    tuning_steps?: number;
    time_signature_num?: number;
    time_signature_den?: number;
}

export type Tool = 'select' | 'pencil' | 'eraser' | 'brush';
export type DragType = 'move' | 'resize' | 'lasso' | 'paint' | 'velocity';
export type ScaleType = 'Major' | 'Minor' | 'Dorian' | 'Phrygian' | 'Lydian' | 'Mixolydian' | 'Locrian' | 'Harmonic Minor' | 'Melodic Minor';
export type ColorMode = 'clip' | 'channel' | 'velocity' | 'pitch';

export interface AudioClipInfo {
    id: string;
    name: string;
    start_sample: number;
    length_samples: number;
    peaks: number[][]; // [level][chunk_index]
}

export interface ParameterInfo {
    id: string;
    name: string;
    value: number;
    min_value: number;
    max_value: number;
}

export interface TrackInfo {
    id: string;
    name: string;
    volume: ParameterInfo;
    is_muted: boolean;
    is_solo: boolean;
    clips: AudioClipInfo[];
    midi_clips: MidiClipInfo[];
}

export interface DragState {
    x: number;
    y: number;
    noteIdx: number;
    type: DragType;
    originalNote: MidiNote | null;
}

export const SCALE_INTERVALS: Record<ScaleType, number[]> = {
    'Major': [0, 2, 4, 5, 7, 9, 11],
    'Minor': [0, 2, 3, 5, 7, 8, 10],
    'Dorian': [0, 2, 3, 5, 7, 9, 10],
    'Phrygian': [0, 1, 3, 5, 7, 8, 10],
    'Lydian': [0, 2, 4, 6, 7, 9, 11],
    'Mixolydian': [0, 2, 4, 5, 7, 9, 10],
    'Locrian': [0, 1, 3, 5, 6, 8, 10],
    'Harmonic Minor': [0, 2, 3, 5, 7, 8, 11],
    'Melodic Minor': [0, 2, 3, 5, 7, 9, 11],
};

export const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

export const BLACK_KEYS = [1, 3, 6, 8, 10]; // C#, D#, F#, G#, A#

export const CHANNEL_COLORS = [
    '#e6194b', '#3cb44b', '#ffe119', '#4363d8', '#f58231', '#911eb4', '#46f0f0', '#f032e6',
    '#bcf60c', '#fabebe', '#008080', '#e6beff', '#9a6324', '#fffac8', '#800000', '#aaffc3'
];

export const GROOVE_TEMPLATES: Record<string, { name: string, timing_offsets: number[], velocity_scale: number[] }> = {
    'Straight': { name: 'Straight', timing_offsets: new Array(16).fill(0), velocity_scale: new Array(16).fill(1) },
    'Swing 16-54': {
        name: 'Swing 16-54',
        timing_offsets: [0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08],
        velocity_scale: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
    },
    'Swing 16-58': {
        name: 'Swing 16-58',
        timing_offsets: [0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16],
        velocity_scale: [1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9]
    },
    'Swing 16-62': {
        name: 'Swing 16-62',
        timing_offsets: [0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24],
        velocity_scale: [1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8]
    },
    'MPC 16-60': {
        name: 'MPC 16-60',
        timing_offsets: [0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20],
        velocity_scale: [1.1, 0.8, 1.05, 0.85, 1.1, 0.8, 1.05, 0.85, 1.1, 0.8, 1.05, 0.85, 1.1, 0.8, 1.05, 0.85]
    }
};

export const getNoteName = (midiNote: number): string => {
    const octave = Math.floor(midiNote / 12) - 1;
    const noteName = NOTE_NAMES[midiNote % 12];
    return `${noteName}${octave}`;
};

