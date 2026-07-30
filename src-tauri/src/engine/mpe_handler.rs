#![allow(dead_code)]
use crate::engine::audio_commands::MidiEvent;
use std::collections::HashMap;

/// MPE (MIDI Polyphonic Expression) Handler
/// Responsible for tracking per-note modulation across multiple channels.
pub struct MpeHandler {
    /// Active notes: (note_number, channel) -> MpeNoteState
    active_notes: HashMap<(u8, u8), MpeNoteState>,
    /// Master channel (usually 0 or 15 - 0-indexed)
    master_channel: u8,
    /// Range of member channels (e.g., 1..15 for Zone 1)
    member_channels: std::ops::RangeInclusive<u8>,
}

#[derive(Clone, Debug)]
pub struct MpeNoteState {
    pub pitch_bend: i16,   // -8192 to 8191
    pub pressure: u8,      // 0-127
    pub timbre: u8,        // CC74, 0-127
    pub lift_velocity: u8, // Release velocity
}

impl MpeHandler {
    pub fn new(master_channel: u8, member_range: std::ops::RangeInclusive<u8>) -> Self {
        Self {
            active_notes: HashMap::new(),
            master_channel,
            member_channels: member_range,
        }
    }

    /// Process an incoming MIDI 1.0 event and update MPE state.
    /// Returns a list of synthesized internal events or modified events.
    pub fn process_event(&mut self, event: &MidiEvent) -> Vec<MpeOutputEvent> {
        let status = event.status & 0xF0;
        let channel = event.status & 0x0F;

        match status {
            0x90 => {
                // Note On
                if self.member_channels.contains(&channel) {
                    let note = event.data1 as u8;
                    let vel = (event.data2 >> 25) as u8;
                    if vel > 0 {
                        self.active_notes.insert(
                            (note, channel),
                            MpeNoteState {
                                pitch_bend: 0,
                                pressure: 0,
                                timbre: 0,
                                lift_velocity: 0,
                            },
                        );
                        return vec![MpeOutputEvent::NoteOn(channel, note, vel)];
                    } else {
                        // Note On with 0 velocity = Note Off
                        self.active_notes.remove(&(note, channel));
                        return vec![MpeOutputEvent::NoteOff(channel, note, 0)];
                    }
                }
            }
            0x80 => {
                // Note Off
                if self.member_channels.contains(&channel) {
                    let note = event.data1 as u8;
                    let vel = (event.data2 >> 25) as u8;
                    self.active_notes.remove(&(note, channel));
                    return vec![MpeOutputEvent::NoteOff(channel, note, vel)];
                }
            }
            0xD0 => {
                // Channel Pressure (Used as poly-pressure in MPE)
                if self.member_channels.contains(&channel) {
                    let pressure = event.data1 as u8;
                    // Apply to all notes on this channel (usually just 1 in MPE)
                    for ((note_num, chan), state) in self.active_notes.iter_mut() {
                        if *chan == channel {
                            state.pressure = pressure;
                            return vec![MpeOutputEvent::Pressure(channel, *note_num, pressure)];
                        }
                    }
                }
            }
            0xE0 => {
                // Pitch Bend
                if self.member_channels.contains(&channel) {
                    // MIDI 1.0 Pitch Bend: 14-bit
                    let lsb = event.data1;
                    let msb = (event.data2 >> 25) as u16;
                    let bend = ((msb << 7) | lsb) as i16 - 8192;

                    for ((note_num, chan), state) in self.active_notes.iter_mut() {
                        if *chan == channel {
                            state.pitch_bend = bend;
                            return vec![MpeOutputEvent::PitchBend(channel, *note_num, bend)];
                        }
                    }
                }
            }
            0xB0 => {
                // Control Change
                let cc = event.data1 as u8;
                let val = (event.data2 >> 25) as u8;

                if cc == 74 && self.member_channels.contains(&channel) {
                    // MPE Timbre (Brightness)
                    for ((note_num, chan), state) in self.active_notes.iter_mut() {
                        if *chan == channel {
                            state.timbre = val;
                            return vec![MpeOutputEvent::Timbre(channel, *note_num, val)];
                        }
                    }
                }
            }
            _ => {}
        }

        vec![]
    }
}

#[derive(Debug, Clone)]
pub enum MpeOutputEvent {
    NoteOn(u8, u8, u8),     // Channel, Note, Velocity
    NoteOff(u8, u8, u8),    // Channel, Note, Velocity
    PitchBend(u8, u8, i16), // Channel, Note, Bend
    Pressure(u8, u8, u8),   // Channel, Note, Val
    Timbre(u8, u8, u8),     // Channel, Note, Val (CC74)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::audio_commands::MidiEvent;

    #[test]
    fn test_mpe_note_tracking() {
        let mut handler = MpeHandler::new(0, 1..=15);

        // Note On on channel 2
        let event_on = MidiEvent {
            sample_offset: 0,
            status: 0x91, // Channel 2 Note On
            data1: 60,
            data2: 100 << 25,
        };

        let outputs = handler.process_event(&event_on);
        assert_eq!(outputs.len(), 1);
        match &outputs[0] {
            MpeOutputEvent::NoteOn(chan, note, vel) => {
                assert_eq!(*chan, 1);
                assert_eq!(*note, 60);
                assert_eq!(*vel, 100);
            }
            _ => panic!("Expected NoteOn"),
        }

        // Pitch Bend on channel 2
        let event_bend = MidiEvent {
            sample_offset: 0,
            status: 0xE1, // Channel 2 Pitch Bend
            data1: 0,
            data2: 0x40 << 25, // Center
        };

        let outputs = handler.process_event(&event_bend);
        assert_eq!(outputs.len(), 1);
        match &outputs[0] {
            MpeOutputEvent::PitchBend(chan, note, bend) => {
                assert_eq!(*chan, 1);
                assert_eq!(*note, 60);
                assert_eq!(*bend, 0);
            }
            _ => panic!("Expected PitchBend"),
        }
    }
}
