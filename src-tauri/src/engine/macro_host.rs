use crate::engine::mod_matrix::ModTarget;

/// Curve types for macro mapping.
#[allow(dead_code)]
pub enum CurveType {
    Linear,
    Logarithmic,
    Exponential,
}

/// A single target destination for a Macro knob.
#[allow(dead_code)]
pub struct MacroTarget {
    pub target: ModTarget,
    pub min_val: f32,
    pub max_val: f32,
    pub curve: CurveType,
}

/// Host for performance macros.
#[allow(dead_code)]
pub struct MacroHost {
    pub targets: Vec<MacroTarget>,
}

#[allow(dead_code)]
impl MacroHost {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
        }
    }

    /// Sets the macro value (0.0 - 1.0) and updates all targets.
    pub fn set_value(&mut self, _value: f32) {
        // Broadcast value with curved mapping
    }
}
