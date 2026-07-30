#![allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UmpType {
    Utility = 0x0,
    SystemRealTime = 0x1,
    Midi1ChannelVoice = 0x2,
    Data64 = 0x3,
    Midi2ChannelVoice = 0x4,
    Data128 = 0x5,
}

#[derive(Debug, Clone, Copy)]
pub struct Ump {
    pub data: [u32; 4],
    pub num_words: usize,
}

impl Ump {
    pub fn new(w0: u32) -> Self {
        Ump {
            data: [w0, 0, 0, 0],
            num_words: 1,
        }
    }

    pub fn from_words(words: &[u32]) -> Self {
        let mut data = [0u32; 4];
        let len = words.len().min(4);
        data[..len].copy_from_slice(&words[..len]);
        Ump {
            data,
            num_words: len,
        }
    }

    pub fn message_type(&self) -> UmpType {
        match (self.data[0] >> 28) & 0xF {
            0x0 => UmpType::Utility,
            0x1 => UmpType::SystemRealTime,
            0x2 => UmpType::Midi1ChannelVoice,
            0x3 => UmpType::Data64,
            0x4 => UmpType::Midi2ChannelVoice,
            0x5 => UmpType::Data128,
            _ => UmpType::Utility, // Fallback
        }
    }
}

pub struct Midi1ToUmp {
    // Basic state for converting byte stream to UMP (MIDI 1.0 Channel Voice wrapped in UMP)
    running_status: u8,
}

impl Midi1ToUmp {
    pub fn new() -> Self {
        Self { running_status: 0 }
    }

    // Convert a complete MIDI 1.0 message (3 bytes usually) to a UMP
    // This is a simplified helper; in a real driver, we'd buffer bytes.
    // For now, assuming midir gives us discrete packets or we parse simple messages.
    pub fn convert_midi1_message(&self, db0: u8, db1: u8, db2: u8, group: u8) -> Ump {
        // MT=0x2 (MIDI 1.0 Channel Voice), Group, Status, Data1, Data2
        let mt = 0x2;
        let word0 = ((mt as u32) << 28)
            | ((group as u32 & 0xF) << 24)
            | ((db0 as u32) << 16)
            | ((db1 as u32) << 8)
            | (db2 as u32);

        Ump::new(word0)
    }
}

// Helper to extract value from MIDI 2.0 UMP
pub fn parse_midi2_cv(ump: &Ump) -> Option<(u8, u8, u16, u32)> {
    // Returns (status, channel, index, value_32bit)
    if ump.message_type() != UmpType::Midi2ChannelVoice {
        return None;
    }

    let word0 = ump.data[0];
    let word1 = ump.data[1];

    let status = ((word0 >> 16) & 0xF0) as u8;
    let channel = ((word0 >> 16) & 0x0F) as u8;

    match status {
        0x80 | 0x90 => {
            // Note On/Off
            let note = ((word0 >> 8) & 0x7F) as u8;
            let velocity = (word1 >> 16) as u16; // Velocity is 16-bit in MIDI 2.0
            Some((status, channel, note as u16, velocity as u32))
        }
        0xB0 => {
            // Control Change - usually it's still 7-bit in spec?
            // Actually MIDI 2.0 uses "Registered Controller" (RPN) style packets for high res usually,
            // but let's check Packet Type 4.
            // Per spec:
            // Status 0xB0 = Control Change.
            // Index = Byte 2. Note logic in MIDI 2.0 is complex.
            // Just assuming 32-bit resolution mapping for now.
            // For MIDI 2.0 RPN/NRPN are preferred for high res.
            // Standard CC in Pkt 4 is still somewhat legacy-ish but fields align.

            // Let's implement RPN (Registered Parameter Number) handling for 32-bit.
            // Status 0x20 is RPN in Pkt 4? No.
            // Let's stick to parsing the value field of generic packets.
            let index = ((word0 >> 8) & 0x7F) as u16;
            let val_data = word1; // 32-bit data
            Some((status, channel, index, val_data))
        }
        _ => None,
    }
}
