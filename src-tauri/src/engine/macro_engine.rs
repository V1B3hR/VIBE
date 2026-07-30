#![allow(dead_code)]
use crate::engine::graph::Parameter;
use std::collections::HashMap;
use uuid::Uuid;

pub struct MacroControl {
    pub id: Uuid,
    pub name: String,
    pub value: f64,
    pub targets: Vec<MacroTarget>,
}

pub struct MacroTarget {
    pub param_id: Uuid,
    pub range_min: f64,
    pub range_max: f64,
    pub curve: f64, // 1.0 = linear
}

pub struct MacroEngine {
    pub macros: HashMap<Uuid, MacroControl>,
}

impl MacroEngine {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
        }
    }

    pub fn set_macro_value(&mut self, macro_id: Uuid, value: f64) {
        if let Some(m) = self.macros.get_mut(&macro_id) {
            m.value = value;
        }
    }

    /// Appplies macro values to a list of parameters
    pub fn apply_to_params(&self, params: &mut Vec<&mut Parameter>) {
        for m in self.macros.values() {
            for target in &m.targets {
                if let Some(param) = params.iter_mut().find(|p| p.id == target.param_id) {
                    let normalized = m.value; // Assume 0.0 - 1.0
                    let scaled = target.range_min
                        + normalized.powf(target.curve) * (target.range_max - target.range_min);
                    param.set_value(scaled);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_scaling() {
        let mut engine = MacroEngine::new();
        let macro_id = Uuid::new_v4();
        let param_id = Uuid::new_v4();

        let m = MacroControl {
            id: macro_id,
            name: "Test Macro".to_string(),
            value: 0.5, // 50%
            targets: vec![MacroTarget {
                param_id,
                range_min: 100.0,
                range_max: 200.0,
                curve: 1.0, // Linear
            }],
        };

        engine.macros.insert(macro_id, m);

        let mut param = Parameter::new("Test Param", 0.0, 0.0, 1000.0);
        param.id = param_id;

        let mut params = vec![&mut param];
        engine.apply_to_params(&mut params);

        assert_eq!(param.get_current_value(), 150.0); // 50% between 100 and 200
    }
}
