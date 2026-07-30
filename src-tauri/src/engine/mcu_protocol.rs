#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VpotMode {
    SingleDot,
    BoostCut,
    Wrap,
    Spread,
}

/// McuDeviceState tracks the complete hardware controller state for a Mackie Control Universal surface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McuDeviceState {
    pub device_name: String,
    pub fader_positions: [u16; 8],     // 14-bit (0..16383)
    pub master_fader_position: u16,    // Master fader
    pub vpot_values: [u8; 8],          // 0..127
    pub vpot_modes: [VpotMode; 8],
    pub is_muted: [bool; 8],
    pub is_solo: [bool; 8],
    pub is_rec_armed: [bool; 8],
    pub lcd_line1: String,             // 56 chars max
    pub lcd_line2: String,             // 56 chars max
    pub assignment_display: String,    // 2 chars (e.g. "PN", "EQ")
}

impl McuDeviceState {
    pub fn new(device_name: &str) -> Self {
        Self {
            device_name: device_name.to_string(),
            fader_positions: [0; 8],
            master_fader_position: 0,
            vpot_values: [64; 8],
            vpot_modes: [VpotMode::SingleDot; 8],
            is_muted: [false; 8],
            is_solo: [false; 8],
            is_rec_armed: [false; 8],
            lcd_line1: "VIBE DAW MCU READY                                      ".to_string(),
            lcd_line2: "TRACK 01-08 SELECTED                                    ".to_string(),
            assignment_display: "PN".to_string(),
        }
    }

    /// Formats a 14-bit motorized fader Pitch Bend MIDI message (0xE0 + channel, LSB, MSB)
    pub fn format_fader_pitch_bend(channel: u8, position_0to1: f64) -> (u8, u8, u8) {
        let channel = channel.min(8); // 0-7 = tracks, 8 = master
        let val_14bit = (position_0to1.clamp(0.0, 1.0) * 16383.0) as u16;
        let lsb = (val_14bit & 0x7F) as u8;
        let msb = ((val_14bit >> 7) & 0x7F) as u8;
        (0xE0 | (channel & 0x0F), lsb, msb)
    }

    /// Formats V-Pot LED Ring CC feedback (CC 48-55)
    pub fn format_vpot_led_ring(channel: u8, val_0to1: f64, mode: VpotMode) -> (u8, u8, u8) {
        let cc = 48 + (channel & 0x07);
        let val_11 = (val_0to1.clamp(0.0, 1.0) * 10.0) as u8 + 1; // 1 to 11

        let mode_bits = match mode {
            VpotMode::SingleDot => 0x00,
            VpotMode::BoostCut => 0x10,
            VpotMode::Wrap => 0x20,
            VpotMode::Spread => 0x30,
        };

        (0xB0, cc, mode_bits | (val_11 & 0x0F))
    }

    /// Update LCD line text padded to 56 characters
    pub fn update_lcd_line(&mut self, line_idx: usize, text: &str) {
        let padded = format!("{:56}", text);
        let truncated = padded[..56].to_string();
        if line_idx == 0 {
            self.lcd_line1 = truncated;
        } else {
            self.lcd_line2 = truncated;
        }
    }
}

/// MCU Event parsed from incoming hardware MIDI message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum McuEvent {
    FaderMove { channel: u8, position_0to1: f64 },
    VpotTurn { channel: u8, delta: i8 },
    ButtonPress { note: u8, is_pressed: bool },
    Unknown,
}

/// Parses incoming MCU hardware MIDI message (status, data1, data2)
pub fn parse_mcu_midi_input(status: u8, data1: u8, data2: u8) -> McuEvent {
    let msg_type = status & 0xF0;
    let channel = status & 0x0F;

    match msg_type {
        // Pitch Bend (Motorized Faders 0-8)
        0xE0 => {
            let val_14bit = (data1 as u16) | ((data2 as u16) << 7);
            let pos_0to1 = (val_14bit as f64) / 16383.0;
            McuEvent::FaderMove {
                channel,
                position_0to1: pos_0to1.clamp(0.0, 1.0),
            }
        }
        // Control Change (V-Pots 16-23)
        0xB0 => {
            if (16..=23).contains(&data1) {
                let vpot_channel = data1 - 16;
                let delta = if data2 & 0x40 != 0 {
                    -((data2 & 0x3F) as i8)
                } else {
                    (data2 & 0x3F) as i8
                };
                McuEvent::VpotTurn {
                    channel: vpot_channel,
                    delta,
                }
            } else {
                McuEvent::Unknown
            }
        }
        // Note On / Note Off (Buttons)
        0x90 | 0x80 => {
            let is_pressed = msg_type == 0x90 && data2 > 0;
            McuEvent::ButtonPress {
                note: data1,
                is_pressed,
            }
        }
        _ => McuEvent::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_fader_pitch_bend() {
        let (status, lsb, msb) = McuDeviceState::format_fader_pitch_bend(0, 0.5);
        assert_eq!(status, 0xE0);
        let val_14bit = (lsb as u16) | ((msb as u16) << 7);
        assert!((val_14bit as i32 - 8191).abs() <= 1);
    }

    #[test]
    fn test_parse_mcu_fader_move() {
        // Pitch bend status 0xE0, LSB 0, MSB 64 (~0.5)
        let event = parse_mcu_midi_input(0xE0, 0, 64);
        if let McuEvent::FaderMove { channel, position_0to1 } = event {
            assert_eq!(channel, 0);
            assert!((position_0to1 - 0.5).abs() < 0.01);
        } else {
            panic!("Expected FaderMove event");
        }
    }

    #[test]
    fn test_format_vpot_led_ring() {
        let (status, cc, data) = McuDeviceState::format_vpot_led_ring(2, 0.5, VpotMode::SingleDot);
        assert_eq!(status, 0xB0);
        assert_eq!(cc, 50);
        assert_eq!(data & 0x0F, 6); // Mid value
    }
}
