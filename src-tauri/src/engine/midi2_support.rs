#![allow(dead_code)]
use crate::engine::midi::{Ump, UmpType};

/// MIDI 2.0 Expression & Parameter Support
/// Handles high-resolution velocity and 32-bit controller data.
#[allow(dead_code)]
pub struct Midi2Engine {
    // State for MIDI 2.0 processing
}

impl Midi2Engine {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {}
    }

    /// Process a MIDI 2.0 Channel Voice Message (MT=0x4)
    #[allow(dead_code)]
    pub fn process_ump(&self, ump: &Ump) -> Option<Midi2Output> {
        if ump.message_type() != UmpType::Midi2ChannelVoice {
            return None;
        }

        let word0 = ump.data[0];
        let word1 = ump.data[1];

        let status = ((word0 >> 16) & 0xF0) as u8;
        let channel = ((word0 >> 16) & 0x0F) as u8;
        let _group = (word0 >> 24) as u8 & 0x0F;

        match status {
            0x80 | 0x90 => {
                // Note Off / Note On
                let note = ((word0 >> 8) & 0x7F) as u8;
                let attribute_type = (word0 & 0xFF) as u8;
                let velocity = (word1 >> 16) as u16; // 16-bit velocity
                let attribute_data = (word1 & 0xFFFF) as u16;

                Some(Midi2Output::Note {
                    on: status == 0x90,
                    channel,
                    note,
                    velocity,
                    attribute_type,
                    attribute_data,
                })
            }
            0xB0 => {
                // Control Change
                let index = ((word0 >> 8) & 0x7F) as u8;
                let value = word1; // 32-bit value
                Some(Midi2Output::ControlChange {
                    channel,
                    index,
                    value,
                })
            }
            0xD0 => {
                // Channel Pressure
                let value = word1; // 32-bit value
                Some(Midi2Output::ChannelPressure { channel, value })
            }
            0xE0 => {
                // Pitch Bend
                let value = word1; // 32-bit value
                Some(Midi2Output::PitchBend { channel, value })
            }
            0x00 => {
                // Registered Controller (RPN)
                let bank = ((word0 >> 8) & 0x7F) as u8;
                let index = (word0 & 0x7F) as u8;
                let value = word1;
                Some(Midi2Output::RPN {
                    channel,
                    bank,
                    index,
                    value,
                })
            }
            _ => None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Midi2Output {
    Note {
        on: bool,
        channel: u8,
        note: u8,
        velocity: u16,
        attribute_type: u8,
        attribute_data: u16,
    },
    ControlChange {
        channel: u8,
        index: u8,
        value: u32,
    },
    ChannelPressure {
        channel: u8,
        value: u32,
    },
    PitchBend {
        channel: u8,
        value: u32, // 32-bit pitch bend
    },
    RPN {
        channel: u8,
        bank: u8,
        index: u8,
        value: u32,
    },
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi2_parsing() {
        let engine = Midi2Engine::new();

        // Note On (Type 4, Status 9)
        // [Message Type: 4, Group: 0, Status: 9, Channel: 0] = 0x40900000
        // [Note: 60, Attribute Type: 0 (None), Velocity: Max] = 0x40903C00
        let ump = Ump::from_words(&[0x40903C00, 0xFFFF0000]);

        let output = engine.process_ump(&ump);
        assert!(output.is_some());
        match output.unwrap() {
            Midi2Output::Note {
                note, velocity, on, ..
            } => {
                assert_eq!(note, 60);
                assert_eq!(velocity, 0xFFFF);
                assert!(on);
            }
            _ => panic!("Expected Note On"),
        }
    }
}
