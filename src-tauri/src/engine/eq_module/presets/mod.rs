use super::dsp::eq_band::{ChannelMode, EqBand, FilterType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EqPreset {
    pub name: String,
    pub bands: Vec<EqBand>,
}

impl EqPreset {
    pub fn new(name: &str, bands: Vec<EqBand>) -> Self {
        Self {
            name: name.to_string(),
            bands,
        }
    }
}

pub fn get_factory_presets() -> Vec<EqPreset> {
    vec![
        EqPreset::new(
            "Default / Flat",
            vec![
                EqBand::new(FilterType::HighPass, 20.0, 0.0, 0.707),
                EqBand::new(FilterType::LowShelf, 100.0, 0.0, 0.707),
                EqBand::new(FilterType::Bell, 1000.0, 0.0, 0.707),
                EqBand::new(FilterType::HighShelf, 5000.0, 0.0, 0.707),
                EqBand::new(FilterType::LowPass, 20000.0, 0.0, 0.707),
            ],
        ),
        EqPreset::new(
            "Vocal Air",
            vec![
                EqBand::new(FilterType::HighPass, 100.0, 0.0, 1.0),
                EqBand::new(FilterType::LowShelf, 200.0, -2.0, 0.707),
                EqBand::new(FilterType::Bell, 3000.0, 2.0, 0.707),
                EqBand::new(FilterType::HighShelf, 10000.0, 4.0, 0.707),
            ],
        ),
        EqPreset::new(
            "Kick Punch",
            vec![
                EqBand::new(FilterType::HighPass, 30.0, 0.0, 1.2),
                EqBand::new(FilterType::Bell, 60.0, 4.0, 2.0),
                EqBand::new(FilterType::Bell, 300.0, -6.0, 1.0),
                EqBand::new(FilterType::HighShelf, 4000.0, 3.0, 0.707),
            ],
        ),
        EqPreset::new(
            "Sub Bass Focus",
            vec![
                EqBand::new(FilterType::LowPass, 150.0, 0.0, 2.0),
                EqBand::new(FilterType::Bell, 50.0, 6.0, 1.0),
            ],
        ),
        EqPreset::new(
            "Master Polish",
            vec![
                EqBand::new(FilterType::HighPass, 20.0, 0.0, 1.0),
                EqBand::new(FilterType::LowShelf, 100.0, 1.0, 0.707),
                EqBand::new(FilterType::Bell, 2500.0, -1.0, 0.5),
                EqBand::new(FilterType::HighShelf, 12000.0, 1.5, 0.707),
            ],
        ),
        EqPreset::new(
            "Male Vocal",
            vec![
                EqBand::new(FilterType::HighPass, 80.0, 0.0, 1.0),
                EqBand::new(FilterType::LowShelf, 200.0, -1.5, 0.707),
                EqBand::new(FilterType::Bell, 3000.0, 2.0, 0.707), // Presence
                EqBand::new(FilterType::HighShelf, 8000.0, 1.0, 0.707),
            ],
        ),
        EqPreset::new(
            "Female Vocal",
            vec![
                EqBand::new(FilterType::HighPass, 120.0, 0.0, 1.0),
                EqBand::new(FilterType::LowShelf, 300.0, -1.0, 0.707),
                EqBand::new(FilterType::Bell, 5000.0, 2.0, 0.707), // Air/Presence
                EqBand::new(FilterType::HighShelf, 10000.0, 2.0, 0.707),
            ],
        ),
        EqPreset::new(
            "Telephone Effect",
            vec![
                EqBand::new(FilterType::HighPass, 500.0, 0.0, 1.0),
                EqBand::new(FilterType::LowPass, 3000.0, 0.0, 1.0),
                EqBand::new(FilterType::Bell, 1000.0, 10.0, 2.0),
            ],
        ),
    ]
}
