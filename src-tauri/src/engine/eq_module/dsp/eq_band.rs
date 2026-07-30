use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FilterType {
    LowPass,
    HighPass,
    Bell,
    LowShelf,
    HighShelf,
    Notch,
    BandPass,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ChannelMode {
    Stereo,
    Left,
    Right,
    Mid,
    Side,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub id: uuid::Uuid,
    pub enabled: bool,
    pub filter_type: FilterType,
    pub freq: f64,
    pub gain_db: f64,
    pub q: f64,
    pub mode: ChannelMode,
    pub solo: bool,
}

impl EqBand {
    pub fn new(filter_type: FilterType, freq: f64, gain_db: f64, q: f64) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            enabled: true,
            filter_type,
            freq,
            gain_db,
            q,
            mode: ChannelMode::Stereo,
            solo: false,
        }
    }
}
