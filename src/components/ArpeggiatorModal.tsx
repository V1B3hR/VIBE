import { invoke } from '@tauri-apps/api/core';

interface ArpeggiatorModalProps {
    show: boolean;
    onClose: () => void;
    clip: any;
    selection: Set<number>;
    trackIdx: number;
    clipId: string;
    loadData: () => void;
    setSelection: (s: Set<number>) => void;
    SAMPLES_PER_BEAT: number;
}

export function ArpeggiatorModal({
    show,
    onClose,
    clip,
    selection,
    trackIdx,
    clipId,
    loadData,
    setSelection,
    SAMPLES_PER_BEAT
}: ArpeggiatorModalProps) {
    if (!show || !clip) return null;

    const handleApply = () => {
        const pattern = (document.getElementById('arp-pattern') as HTMLSelectElement).value;
        const rate = parseInt((document.getElementById('arp-rate') as HTMLSelectElement).value);
        const octaves = parseInt((document.getElementById('arp-octaves') as HTMLInputElement).value);
        const gate = parseInt((document.getElementById('arp-gate') as HTMLInputElement).value) / 100;

        if (selection.size === 0) {
            alert('Please select notes first');
            return;
        }

        // Generate arpeggiated notes
        const selectedNotes = Array.from(selection).map(idx => clip.notes[idx]);
        const uniquePitches = [...new Set(selectedNotes.map((n: any) => n.note))].sort((a, b) => a - b);

        // Expand pitches across octaves
        const expandedPitches: number[] = [];
        for (let oct = 0; oct < octaves; oct++) {
            uniquePitches.forEach(p => expandedPitches.push(p + (oct * 12)));
        }

        // Apply pattern
        let arpPattern: number[] = [];
        if (pattern === 'up') arpPattern = expandedPitches;
        else if (pattern === 'down') arpPattern = [...expandedPitches].reverse();
        else if (pattern === 'updown') arpPattern = [...expandedPitches, ...[...expandedPitches].reverse().slice(1, -1)];
        else if (pattern === 'random') arpPattern = expandedPitches.sort(() => Math.random() - 0.5);
        else if (pattern === 'chord') arpPattern = expandedPitches;

        // Calculate timing
        const startSample = Math.min(...selectedNotes.map((n: any) => n.start_sample));
        const samplesPerNote = (SAMPLES_PER_BEAT * 4) / rate;
        const noteLength = Math.floor(samplesPerNote * gate);

        // Delete original selected notes
        const toDelete = Array.from(selection).sort((a, b) => b - a);
        toDelete.forEach(idx => {
            invoke('delete_midi_note', { trackIdx, clipId, noteIdx: idx });
        });

        // Create arpeggiated notes
        if (pattern === 'chord') {
            arpPattern.forEach(pitch => {
                const newNote = {
                    start_sample: startSample,
                    length_samples: noteLength,
                    note: pitch,
                    velocity: Math.floor(100 * 33818640),
                    channel: 0,
                    probability: 1.0,
                    velocity_random: 0,
                    timing_random: 0
                };
                invoke('add_midi_note', { trackIdx, clipId, note: newNote });
            });
        } else {
            arpPattern.forEach((pitch, i) => {
                const newNote = {
                    start_sample: startSample + (i * samplesPerNote),
                    length_samples: noteLength,
                    note: pitch,
                    velocity: Math.floor(100 * 33818640),
                    channel: 0,
                    probability: 1.0,
                    velocity_random: 0,
                    timing_random: 0
                };
                invoke('add_midi_note', { trackIdx, clipId, note: newNote });
            });
        }

        setTimeout(() => {
            loadData();
            setSelection(new Set());
            onClose();
        }, 100);
    };

    return (
        <div className="arpeggiator-modal-overlay" onClick={onClose}>
            <div className="arpeggiator-modal" onClick={(e) => e.stopPropagation()}>
                <h3>🎹 ARPEGGIATOR</h3>

                <div className="arp-section">
                    <label>Pattern</label>
                    <select id="arp-pattern" className="arp-select">
                        <option value="up">Up</option>
                        <option value="down">Down</option>
                        <option value="updown">Up-Down</option>
                        <option value="random">Random</option>
                        <option value="chord">Chord</option>
                    </select>
                </div>

                <div className="arp-section">
                    <label>Rate</label>
                    <select id="arp-rate" className="arp-select">
                        <option value="4">1/4</option>
                        <option value="8">1/8</option>
                        <option value="16">1/16</option>
                        <option value="32">1/32</option>
                    </select>
                </div>

                <div className="arp-section">
                    <label>Octaves</label>
                    <input id="arp-octaves" type="number" min="1" max="4" defaultValue="1" className="arp-input" />
                </div>

                <div className="arp-section">
                    <label>Gate (%)</label>
                    <input id="arp-gate" type="number" min="10" max="100" defaultValue="80" className="arp-input" />
                </div>

                <div className="arp-buttons">
                    <button className="arp-btn arp-apply" onClick={handleApply}>APPLY</button>
                    <button className="arp-btn arp-cancel" onClick={onClose}>CANCEL</button>
                </div>
            </div>
        </div>
    );
}
