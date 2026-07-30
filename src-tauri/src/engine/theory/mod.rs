#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chord {
    pub root: u8,            // 0-11
    pub quality: String,     // "Maj", "Min", "Dim", "Aug", "7", "Maj7", "min7", "m7b5"
    pub extensions: Vec<u8>, // Extra intervals (9, 11, 13)
}

impl Chord {
    pub fn new(root: u8, quality: &str) -> Self {
        Self {
            root,
            quality: quality.to_string(),
            extensions: vec![],
        }
    }

    /// Returns the intervals for this chord quality
    pub fn get_intervals(&self) -> Vec<u8> {
        match self.quality.as_str() {
            "Maj" => vec![0, 4, 7],
            "Min" => vec![0, 3, 7],
            "Dim" => vec![0, 3, 6],
            "Aug" => vec![0, 4, 8],
            "7" => vec![0, 4, 7, 10],
            "Maj7" => vec![0, 4, 7, 11],
            "min7" => vec![0, 3, 7, 10],
            "m7b5" => vec![0, 3, 6, 10], // Half-diminished
            _ => vec![0, 4, 7],          // Default to major
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    pub root: String,       // "C", "C#", ...
    pub scale_type: String, // "Major", "Minor"
}

impl Key {
    /// Returns the neighboring keys based on Circle of Fifths
    pub fn neighbors(&self) -> Vec<Key> {
        let roots = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let root_idx = roots.iter().position(|&r| r == self.root).unwrap_or(0);

        let mut neighbors = Vec::new();

        // 1. Dominant (Clockwise, +7 semitones)
        let dom_idx = (root_idx + 7) % 12;
        neighbors.push(Key {
            root: roots[dom_idx].to_string(),
            scale_type: self.scale_type.clone(),
        });

        // 2. Subdominant (Counter-Clockwise, -7 == +5 semitones)
        let sub_idx = (root_idx + 5) % 12;
        neighbors.push(Key {
            root: roots[sub_idx].to_string(),
            scale_type: self.scale_type.clone(),
        });

        // 3. Relative Key
        let rel_idx = if self.scale_type == "Major" {
            (root_idx + 9) % 12 // C Maj -> A Min (-3 semitones / +9)
        } else {
            (root_idx + 3) % 12 // A Min -> C Maj (+3 semitones)
        };
        neighbors.push(Key {
            root: roots[rel_idx].to_string(),
            scale_type: if self.scale_type == "Major" {
                "Minor".to_string()
            } else {
                "Major".to_string()
            },
        });

        neighbors
    }
}

pub mod generator;
pub use generator::Generator;
